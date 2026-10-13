use std::path::PathBuf;
use std::sync::RwLock;

use crate::config::AppSettings;
use crate::git::Repository;
use crate::process::Processes;
use crate::secrets::Secrets;
use crate::terminal::Terminals;
use crate::watch::Watcher;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

pub trait Context: Send + Sync + 'static {
    type Repository: Repository;
    type Secrets: Secrets;
    type Processes: Processes;
    type Watcher: Watcher;
    type Terminals: Terminals;

    fn paths(&self) -> &AppPaths;
    fn settings(&self) -> &RwLock<AppSettings>;
    fn repository(&self) -> &Self::Repository;
    fn secrets(&self) -> &Self::Secrets;
    fn processes(&self) -> &Self::Processes;
    fn watcher(&self) -> &Self::Watcher;
    fn terminals(&self) -> &Self::Terminals;
}
