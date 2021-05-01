use reshare_models::FileInfo;

pub enum StorageState {
    Public,
    Private { key_phrase: String },
}

impl std::fmt::Display for StorageState {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StorageState::Public => write!(fmt, "Contents of the public storage"),
            StorageState::Private { key_phrase } => write!(fmt, "{}", key_phrase),
        }
    }
}

pub struct AppState {
    pub storage_state: StorageState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            storage_state: StorageState::Public,
        }
    }
}

impl AppState {
    pub fn fetch_files(&self) -> Result<Vec<FileInfo>, ()> {
        match &self.storage_state {
            StorageState::Public => Ok(vec![FileInfo::dummy()]),
            StorageState::Private { .. } => Ok(vec![]),
        }
    }

    pub fn download_url_root(&self) -> String {
        match &self.storage_state {
            StorageState::Public => "/api/download/".to_owned(),
            StorageState::Private { key_phrase } => format!("/api/private/{}/", key_phrase),
        }
    }
}
