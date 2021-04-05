use reshare_models::FileInfo;
use std::collections::{hash_set::Iter, HashMap, HashSet};

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

    pub fn add_file(&mut self, file_info: FileInfo, keyphrase: Option<String>) {
        match keyphrase {
            Some(key) => self.private.add_file(key, file_info),
            None => self.public.add_file(file_info),
        }
    }

    pub fn list(&self, keyphrase: &Option<String>) -> Option<impl Iterator<Item = &FileInfo>> {
        // Why can't rust compiler infer correct types when I use impl Iterator in list definitions?
        match keyphrase {
            Some(key) => self.private.list(key),
            None => Some(self.public.list()),
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

    fn is_file_exists(&self, file_info: &FileInfo) -> bool {
        self.0.contains(file_info)
    }

    fn add_file(&mut self, file_info: FileInfo) {
        self.0.insert(file_info);
    }

    fn list(&self) -> Iter<'_, FileInfo> {
        self.0.iter()
    }
}

#[derive(Debug, Clone)]
struct PrivateStorage(HashMap<String, Storage>);

impl PrivateStorage {
    fn new() -> Self {
        Self(HashMap::new())
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

    fn list(&self, shard_name: &str) -> Option<Iter<'_, FileInfo>> {
        self.0.get(shard_name).map(|storage| storage.iter())
    }
}
