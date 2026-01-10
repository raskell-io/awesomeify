//! Rate limiting with exponential backoff for GitHub API

use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

use super::graphql::RateLimit;

/// Rate limiter that tracks GitHub API limits and handles backoff
#[derive(Debug, Clone)]
pub struct RateLimiter {
    state: Arc<RwLock<RateLimitState>>,
    min_remaining: i64,
}

#[derive(Debug)]
struct RateLimitState {
    remaining: i64,
    reset_at: Option<DateTime<Utc>>,
    consecutive_errors: u32,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// `min_remaining` is the minimum number of requests to keep in reserve
    pub fn new(min_remaining: i64) -> Self {
        Self {
            state: Arc::new(RwLock::new(RateLimitState {
                remaining: 5000, // GitHub default
                reset_at: None,
                consecutive_errors: 0,
            })),
            min_remaining,
        }
    }

    /// Update the rate limit state from a response
    pub async fn update(&self, rate_limit: &RateLimit) {
        let mut state = self.state.write().await;
        state.remaining = rate_limit.remaining;
        state.reset_at = DateTime::parse_from_rfc3339(&rate_limit.reset_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));
        state.consecutive_errors = 0;

        tracing::debug!(
            remaining = rate_limit.remaining,
            reset_at = %rate_limit.reset_at,
            "Rate limit updated"
        );
    }

    /// Record an error and return backoff duration
    pub async fn record_error(&self) -> Duration {
        let mut state = self.state.write().await;
        state.consecutive_errors += 1;

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, max 60s
        let backoff_secs = (2_u64.pow(state.consecutive_errors.min(6) - 1)).min(60);

        tracing::warn!(
            consecutive_errors = state.consecutive_errors,
            backoff_secs = backoff_secs,
            "Recording error, will backoff"
        );

        Duration::from_secs(backoff_secs)
    }

    /// Wait if necessary before making a request
    pub async fn wait_if_needed(&self) {
        let state = self.state.read().await;

        // Check if we're below the threshold
        if state.remaining <= self.min_remaining {
            if let Some(reset_at) = state.reset_at {
                let now = Utc::now();
                if reset_at > now {
                    let wait_duration = (reset_at - now).to_std().unwrap_or(Duration::from_secs(60));

                    tracing::info!(
                        remaining = state.remaining,
                        wait_secs = wait_duration.as_secs(),
                        "Rate limit low, waiting for reset"
                    );

                    drop(state); // Release lock before sleeping
                    sleep(wait_duration).await;
                }
            }
        }

        // Add a small delay between requests to be nice to the API
        sleep(Duration::from_millis(100)).await;
    }

    /// Get current remaining requests
    pub async fn remaining(&self) -> i64 {
        self.state.read().await.remaining
    }
}

/// Retry a future with exponential backoff
pub async fn with_retry<F, Fut, T, E>(
    rate_limiter: &RateLimiter,
    max_retries: u32,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;

    for attempt in 0..=max_retries {
        rate_limiter.wait_if_needed().await;

        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!(
                    attempt = attempt + 1,
                    max_retries = max_retries,
                    error = %e,
                    "Request failed, retrying"
                );

                last_error = Some(e);

                if attempt < max_retries {
                    let backoff = rate_limiter.record_error().await;
                    sleep(backoff).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
