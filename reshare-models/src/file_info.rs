use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub upload_date: DateTime<Local>,

    #[serde(skip_serializing, skip_deserializing)]
    pub storage_path: std::path::PathBuf,
}

impl FileInfo {
    pub fn dummy() -> Self {
        Self {
            name: "dummy-file-name.jpg".to_string(),
            size: 100500,
            upload_date: Local::now(),
            storage_path: "/".into(),
        }
    }
}

impl std::cmp::PartialEq for FileInfo {
    fn eq(&self, rhs: &Self) -> bool {
        self.name.eq(&rhs.name)
    }
}

impl std::cmp::Eq for FileInfo {}

impl std::hash::Hash for FileInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
