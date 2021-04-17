use reshare_models::FileInfo;
use std::collections::{hash_set::Iter, HashMap, HashSet};
use thiserror::Error;

pub type Result<T, E = StorageError> = std::result::Result<T, E>;

#[derive(Debug, Clone)]
pub struct FileStorage {
    public: PublicStorage,
    private: PrivateStorage,
}

impl FileStorage {
    pub fn new() -> Self {
        Self {
            public: PublicStorage::new(),
            private: PrivateStorage::new(),
        }
    }

    pub fn is_file_exists(&self, file_info: &FileInfo, keyphrase: &Option<String>) -> bool {
        match keyphrase {
            Some(key) => self.private.is_file_exists(key, file_info),
            None => self.public.is_file_exists(file_info),
        }
    }

    pub fn get_file(&self, file_name: String, keyphrase: &Option<String>) -> Option<&FileInfo> {
        let file_info = FileInfo::from_name(file_name);

        match keyphrase {
            Some(key) => self.private.get_file(key, &file_info),
            None => self.public.get_file(&file_info),
        }
    }

    pub fn add_file(&mut self, file_info: FileInfo, keyphrase: Option<String>) {
        match keyphrase {
            Some(key) => self.private.add_file(key, file_info),
            None => self.public.add_file(file_info),
        }
    }

    pub fn list(&self, keyphrase: &Option<String>) -> Result<impl Iterator<Item = &FileInfo>> {
        match keyphrase {
            Some(key) => self.private.list(key).ok_or(StorageError::DoesntExist),
            None => Ok(self.public.list()),
        }
    }
}

type Storage = HashSet<FileInfo>;

#[derive(Debug, Clone)]
struct PublicStorage(Storage);

impl PublicStorage {
    fn new() -> Self {
        Self(Storage::new())
    }

    fn list(&self) -> Iter<'_, FileInfo> {
        self.0.iter()
    }

    fn is_file_exists(&self, file_info: &FileInfo) -> bool {
        self.0.contains(file_info)
    }

    fn get_file(&self, file_info: &FileInfo) -> Option<&FileInfo> {
        self.0.get(file_info)
    }

    fn add_file(&mut self, file_info: FileInfo) {
        self.0.insert(file_info);
    }
}

#[derive(Debug, Clone)]
struct PrivateStorage(HashMap<String, Storage>);

impl PrivateStorage {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn list(&self, shard_name: &str) -> Option<Iter<'_, FileInfo>> {
        self.0.get(shard_name).map(|storage| storage.iter())
    }

    fn get_file(&self, shard_name: &str, file_info: &FileInfo) -> Option<&FileInfo> {
        self.0
            .get(shard_name)
            .and_then(|storage| storage.get(file_info))
    }

    fn is_file_exists(&self, shard_name: &str, file_info: &FileInfo) -> bool {
        self.0
            .get(shard_name)
            .map(|storage| storage.contains(file_info))
            .unwrap_or(false)
    }

    fn add_file(&mut self, shard_name: String, file_info: FileInfo) {
        let storage = self.0.entry(shard_name).or_insert(Storage::new());
        storage.insert(file_info);
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Requested storage doesn't exist")]
    DoesntExist,
}

impl actix_web::error::ResponseError for StorageError {
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::dev::HttpResponseBuilder::new(self.status_code()).json(reshare_models::Error {
            error_msg: self.to_string(),
        })
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::NOT_FOUND
    }
}
