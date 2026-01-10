//! Static site generator for Awesomeify

mod json_export;
mod site;

pub use json_export::export_json;
pub use site::{GeneratorConfig, SiteGenerator};
