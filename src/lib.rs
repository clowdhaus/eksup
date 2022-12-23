mod cli;
pub use cli::{Commands, Compute, Playbook, Upgrade, LATEST as LatestVersion};

mod data;
pub use data::TemplateData;
