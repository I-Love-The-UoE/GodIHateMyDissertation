use std::path::PathBuf;

use gihmd_core::AppPaths;
use gihmd_core::config::AppSettings;
use gihmd_core::config::ProjectRegistry;
use gihmd_core::config::load_settings;
use tauri::Manager;

use crate::error::StartupError;

#[derive(Debug)]
pub struct Config {
    pub paths: AppPaths,
    pub log_dir: PathBuf,
    pub settings: AppSettings,
    pub projects: ProjectRegistry,
}

/// Called once from setup; every failure aborts start-up.
pub fn load(app: &tauri::App) -> Result<Config, StartupError> {
    let resolver = app.path();
    let config_dir = resolver.app_config_dir()?;
    let cache_dir = resolver.app_cache_dir()?;
    let data_dir = resolver.app_data_dir()?;
    let log_dir = resolver.app_log_dir()?;
    let settings = tauri::async_runtime::block_on(load_settings(&config_dir))?;
    let projects = tauri::async_runtime::block_on(ProjectRegistry::load(&config_dir))?;

    Ok(Config {
        paths: AppPaths {
            config_dir,
            cache_dir,
            data_dir,
        },
        log_dir,
        settings,
        projects,
    })
}
