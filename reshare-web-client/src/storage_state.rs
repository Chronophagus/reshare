#[derive(Debug, Clone)]
pub enum StorageState {
    Public,
    Private { key_phrase: String },
}

impl StorageState {
    pub fn fetch_files_url(&self) -> String {
        match &self {
            StorageState::Public => "/api/list".to_owned(),
            StorageState::Private { key_phrase } => {
                format!("/api/private/{}", urlencoding::encode(key_phrase))
            }
        }
    }

    pub fn download_url_root(&self) -> String {
        match &self {
            StorageState::Public => "/api/download/".to_owned(),
            StorageState::Private { key_phrase } => {
                format!("/api/private/{}/", urlencoding::encode(key_phrase))
            }
        }
    }
}

impl std::fmt::Display for StorageState {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StorageState::Public => write!(fmt, "Contents of the public storage"),
            StorageState::Private { key_phrase } => write!(fmt, "{}", key_phrase),
        }
    }
}
