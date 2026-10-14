use std::sync::RwLock;

use gihmd_core::AppPaths;
use gihmd_core::Context;
use gihmd_core::config::AppSettings;
use gihmd_fetch::ReqwestDownloads;
use gihmd_git::Git2Repository;
use gihmd_process::TokioProcesses;
use gihmd_pty::PortableTerminals;
use gihmd_secrets::KeyringSecrets;
use gihmd_watch::NotifyWatcher;

#[derive(Debug)]
pub struct AppContext {
    paths: AppPaths,
    settings: RwLock<AppSettings>,
    repository: Git2Repository,
    secrets: KeyringSecrets,
    processes: std::sync::Arc<TokioProcesses>,
    watcher: NotifyWatcher,
    terminals: PortableTerminals,
    downloads: ReqwestDownloads,
}

impl AppContext {
    pub fn new(paths: AppPaths, settings: AppSettings) -> Self {
        Self {
            paths,
            settings: RwLock::new(settings),
            repository: gihmd_git::default(),
            secrets: gihmd_secrets::default(),
            processes: std::sync::Arc::new(gihmd_process::default()),
            watcher: gihmd_watch::default(),
            terminals: gihmd_pty::default(),
            downloads: gihmd_fetch::default(),
        }
    }

    pub fn shared_processes(&self) -> std::sync::Arc<TokioProcesses> {
        std::sync::Arc::clone(&self.processes)
    }
}

impl Context for AppContext {
    type Repository = Git2Repository;
    type Secrets = KeyringSecrets;
    type Processes = TokioProcesses;
    type Watcher = NotifyWatcher;
    type Terminals = PortableTerminals;
    type Downloads = ReqwestDownloads;

    fn paths(&self) -> &AppPaths {
        &self.paths
    }

    fn settings(&self) -> &RwLock<AppSettings> {
        &self.settings
    }

    fn repository(&self) -> &Self::Repository {
        &self.repository
    }

    fn secrets(&self) -> &Self::Secrets {
        &self.secrets
    }

    fn processes(&self) -> &Self::Processes {
        &self.processes
    }

    fn watcher(&self) -> &Self::Watcher {
        &self.watcher
    }

    fn terminals(&self) -> &Self::Terminals {
        &self.terminals
    }

    fn downloads(&self) -> &Self::Downloads {
        &self.downloads
    }
}
