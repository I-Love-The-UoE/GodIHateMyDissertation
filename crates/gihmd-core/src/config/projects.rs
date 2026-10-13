use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::config::ConfigError;
use crate::paths::RelPath;

const PROJECTS_FILE: &str = "projects.json";
const RECENT_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: Uuid,
    pub root: PathBuf,
    pub main_file: Option<RelPath>,
    pub last_opened: SystemTime,
    pub open_tabs: Vec<RelPath>,
    pub active_tab: Option<RelPath>,
    pub claude_session_id: Option<Uuid>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRegistry {
    records: Vec<ProjectRecord>,
}

impl ProjectRegistry {
    pub async fn load(config_dir: &Path) -> Result<Self, ConfigError> {
        let path = config_dir.join(PROJECTS_FILE);

        let text = match tokio::fs::read_to_string(&path).await {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => return Err(ConfigError::Io { path, source }),
        };

        serde_json::from_str(&text).map_err(|source| ConfigError::Malformed { path, source })
    }

    pub async fn save(&self, config_dir: &Path) -> Result<(), ConfigError> {
        let path = config_dir.join(PROJECTS_FILE);
        tokio::fs::create_dir_all(config_dir)
            .await
            .map_err(|source| ConfigError::Io {
                path: config_dir.to_path_buf(),
                source,
            })?;
        let text = serde_json::to_string_pretty(self).map_err(|source| ConfigError::Malformed {
            path: path.clone(),
            source,
        })?;

        tokio::fs::write(&path, text)
            .await
            .map_err(|source| ConfigError::Io { path, source })
    }

    pub fn recent(&self) -> Vec<ProjectRecord> {
        let mut records = self.records.clone();
        records.sort_by_key(|record| std::cmp::Reverse(record.last_opened));
        records.truncate(RECENT_LIMIT);

        records
    }

    pub fn find(&self, root: &Path) -> Option<&ProjectRecord> {
        self.records.iter().find(|record| record.root == root)
    }

    /// Returns the record for `root`, creating it on first open, and marks it as just opened.
    pub fn touch(&mut self, root: &Path) -> &mut ProjectRecord {
        let position = self.records.iter().position(|record| record.root == root);

        let index = if let Some(index) = position {
            index
        } else {
            self.records.push(ProjectRecord {
                id: Uuid::new_v4(),
                root: root.to_path_buf(),
                main_file: None,
                last_opened: SystemTime::now(),
                open_tabs: Vec::new(),
                active_tab: None,
                claude_session_id: None,
            });

            self.records.len() - 1
        };

        let record = &mut self.records[index];
        record.last_opened = SystemTime::now();

        record
    }

    pub fn update(&mut self, updated: ProjectRecord) {
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.id == updated.id)
        {
            *record = updated;
        }
    }

    pub fn forget(&mut self, root: &Path) {
        self.records.retain(|record| record.root != root);
    }
}
