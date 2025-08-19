use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub parent: Option<String>,
    pub author: String,
    pub committer: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub tree: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub hash: String,
    pub is_file: bool,
    pub mode: String, // File permissions/type
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Index {
    pub entries: HashMap<String, IndexEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexEntry {
    pub hash: String,
    pub mode: String,
    pub size: u64,
    pub mtime: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ref {
    pub name: String,
    pub hash: String,
    pub ref_type: RefType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RefType {
    Branch,
    Tag,
    Remote,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackFile {
    pub objects: Vec<PackedObject>,
    pub checksum: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackedObject {
    pub hash: String,
    pub object_type: ObjectType,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl Index {
    pub fn new() -> Self {
        Index {
            entries: HashMap::new(),
        }
    }

    pub fn load() -> io::Result<Self> {
        let index_path = if Path::new(".bloc").exists() {
            ".bloc/index"
        } else {
            "index" // For bare repositories
        };
        
        if Path::new(index_path).exists() {
            let content = fs::read_to_string(index_path)?;
            serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        } else {
            Ok(Index::new())
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let index_path = if Path::new(".bloc").exists() {
            ".bloc/index"
        } else {
            "index"
        };
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(index_path, content)
    }

    pub fn add_entry(&mut self, path: String, hash: String, size: u64) {
        let entry = IndexEntry {
            hash,
            mode: "100644".to_string(), // Regular file
            size,
            mtime: Utc::now(),
        };
        self.entries.insert(path, entry);
    }

    pub fn remove_entry(&mut self, path: &str) -> bool {
        self.entries.remove(path).is_some()
    }

    pub fn is_staged(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }

    pub fn get_staged_files(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }
}

impl Commit {
    pub fn new(
        parent: Option<String>,
        author: String,
        message: String,
        tree: String,
    ) -> Self {
        Commit {
            parent,
            author: author.clone(),
            committer: author, // For now, author and committer are the same
            timestamp: Utc::now(),
            message,
            tree,
        }
    }
}

impl TreeEntry {
    pub fn new_file(name: String, hash: String) -> Self {
        TreeEntry {
            name,
            hash,
            is_file: true,
            mode: "100644".to_string(),
        }
    }

    pub fn new_directory(name: String, hash: String) -> Self {
        TreeEntry {
            name,
            hash,
            is_file: false,
            mode: "040000".to_string(),
        }
    }
}

impl PackFile {
    pub fn new() -> Self {
        PackFile {
            objects: Vec::new(),
            checksum: String::new(),
        }
    }

    pub fn add_object(&mut self, hash: String, object_type: ObjectType, data: Vec<u8>) {
        let packed_object = PackedObject {
            hash,
            object_type,
            data,
        };
        self.objects.push(packed_object);
    }

    pub fn finalize(&mut self) {
        // Calculate checksum of all objects
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for obj in &self.objects {
            hasher.update(&obj.data);
        }
        self.checksum = format!("{:x}", hasher.finalize());
    }
}
