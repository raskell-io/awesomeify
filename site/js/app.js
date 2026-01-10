/**
 * Awesomeify - Client-side filtering and sorting
 */

document.addEventListener('DOMContentLoaded', () => {
    initFilters();
    initSorting();
});

/**
 * Initialize filter controls
 */
function initFilters() {
    const languageFilter = document.getElementById('language-filter');
    const activeOnly = document.getElementById('active-only');
    const hideForks = document.getElementById('hide-forks');

    if (languageFilter) {
        languageFilter.addEventListener('change', applyFilters);
    }

    if (activeOnly) {
        activeOnly.addEventListener('change', applyFilters);
    }

    if (hideForks) {
        hideForks.addEventListener('change', applyFilters);
    }
}

/**
 * Initialize sorting controls
 */
function initSorting() {
    const sortBy = document.getElementById('sort-by');

    if (sortBy) {
        sortBy.addEventListener('change', () => {
            sortRepos(sortBy.value);
        });
    }
}

/**
 * Apply all active filters
 */
function applyFilters() {
    const reposList = document.getElementById('repos-list');
    if (!reposList) return;

    const repos = Array.from(reposList.querySelectorAll('.repo-card'));
    const languageFilter = document.getElementById('language-filter');
    const activeOnly = document.getElementById('active-only');
    const hideForks = document.getElementById('hide-forks');

    const selectedLang = languageFilter?.value?.toLowerCase() || '';
    const showActiveOnly = activeOnly?.checked || false;
    const shouldHideForks = hideForks?.checked || false;

    let visibleCount = 0;

    repos.forEach(repo => {
        let visible = true;

        // Language filter
        if (selectedLang) {
            const repoLang = (repo.dataset.language || '').toLowerCase();
            if (repoLang !== selectedLang) {
                visible = false;
            }
        }

        // Active only filter
        if (showActiveOnly && visible) {
            const activity = parseFloat(repo.dataset.activity) || 0;
            if (activity < 50) {
                visible = false;
            }
        }

        // Hide forks filter
        if (shouldHideForks && visible) {
            const isFork = repo.dataset.fork === 'true';
            if (isFork) {
                visible = false;
            }
        }

        repo.style.display = visible ? '' : 'none';
        if (visible) visibleCount++;
    });

    // Update visible count
    const countEl = document.getElementById('visible-count');
    if (countEl) {
        countEl.textContent = visibleCount;
    }
}

/**
 * Sort repositories by the given field
 */
function sortRepos(sortField) {
    const reposList = document.getElementById('repos-list');
    if (!reposList) return;

    const repos = Array.from(reposList.querySelectorAll('.repo-card'));

    repos.sort((a, b) => {
        switch (sortField) {
            case 'stars':
                return parseInt(b.dataset.stars || 0) - parseInt(a.dataset.stars || 0);

            case 'activity':
                return parseFloat(b.dataset.activity || 0) - parseFloat(a.dataset.activity || 0);

            case 'recent':
                const dateA = new Date(a.dataset.pushed || 0);
                const dateB = new Date(b.dataset.pushed || 0);
                return dateB - dateA;

            case 'name':
                const nameA = a.querySelector('.repo-name')?.textContent?.toLowerCase() || '';
                const nameB = b.querySelector('.repo-name')?.textContent?.toLowerCase() || '';
                return nameA.localeCompare(nameB);

            default:
                return 0;
        }
    });

    // Re-append in sorted order
    repos.forEach(repo => reposList.appendChild(repo));
}

/**
 * Format a number with commas
 */
function formatNumber(n) {
    return n.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

/**
 * Debounce function for performance
 */
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}
