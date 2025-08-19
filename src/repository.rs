use crate::config::BlocConfig;
use crate::objects::{Commit, TreeEntry, Index};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use walkdir::WalkDir;
use colored::*;

pub struct BlocRepo {
    pub config: BlocConfig,
    pub index: Index,
    pub is_bare: bool,
    pub work_dir: PathBuf,
    pub bloc_dir: PathBuf,
}

impl BlocRepo {
    pub fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let bloc_dir = current_dir.join(".bloc");
        
        if !bloc_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Not a bloc repository"
            ));
        }

        let config = BlocConfig::load()?;
        let index = Index::load()?;
        let is_bare = config.core.bare;

        Ok(BlocRepo {
            config,
            index,
            is_bare,
            work_dir: current_dir,
            bloc_dir,
        })
    }

    pub fn init(path: Option<&str>, bare: bool) -> io::Result<Self> {
        let work_dir = if let Some(path) = path {
            let p = PathBuf::from(path);
            if !p.exists() {
                fs::create_dir_all(&p)?;
            }
            std::env::set_current_dir(&p)?;
            p
        } else {
            std::env::current_dir()?
        };

        let bloc_dir = if bare {
            work_dir.clone()
        } else {
            work_dir.join(".bloc")
        };

        if bloc_dir.join("HEAD").exists() || bloc_dir.join("config").exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Repository already exists"
            ));
        }

        // Create directory structure
        if !bare {
            fs::create_dir(&bloc_dir)?;
        }
        
        fs::create_dir_all(bloc_dir.join("objects"))?;
        fs::create_dir_all(bloc_dir.join("refs/heads"))?;
        fs::create_dir_all(bloc_dir.join("refs/tags"))?;
        fs::create_dir_all(bloc_dir.join("refs/remotes"))?;

        // Create config
        let mut config = BlocConfig::default();
        config.core.bare = bare;
        
        // Save config
        std::env::set_current_dir(&work_dir)?;
        if bare {
            let content = serde_json::to_string_pretty(&config)?;
            fs::write("config", content)?;
        } else {
            config.save()?;
        }

        // Create HEAD
        let head_content = format!("ref: refs/heads/{}\n", config.core.default_branch);
        let head_path = if bare { "HEAD" } else { ".bloc/HEAD" };
        fs::write(head_path, head_content)?;

        // Create index for non-bare repos
        if !bare {
            let index = Index::new();
            index.save()?;
        }

        // Hide .bloc directory on Windows (for non-bare repos)
        #[cfg(windows)]
        if !bare {
            let _ = crate::hide_directory(".bloc");
        }

        let repo = BlocRepo {
            config,
            index: if bare { Index::new() } else { Index::load()? },
            is_bare: bare,
            work_dir: work_dir.clone(),
            bloc_dir: bloc_dir.clone(),
        };

        if bare {
            println!("{} {} {}", 
                     "Initialized empty bare Bloc repository in".bright_green().bold(),
                     work_dir.display().to_string().bright_cyan(),
                     "🎉".bright_green());
        } else {
            println!("{} {} {}", 
                     "Initialized empty Bloc repository in".bright_green().bold(),
                     bloc_dir.display().to_string().bright_cyan(),
                     "🎉".bright_green());
            println!("{}", "(.bloc directory is now hidden)".bright_black());
        }

        Ok(repo)
    }

    pub fn is_repo() -> bool {
        Path::new(".bloc").exists() || 
        (Path::new("HEAD").exists() && Path::new("config").exists())
    }

    pub fn is_bare_repo() -> bool {
        Path::new("HEAD").exists() && Path::new("config").exists() && !Path::new(".bloc").exists()
    }

    pub fn get_current_branch(&self) -> io::Result<String> {
        let head_path = if self.is_bare { "HEAD" } else { ".bloc/HEAD" };
        let head_content = fs::read_to_string(head_path)?;
        
        if head_content.starts_with("ref: ") {
            let branch_ref = head_content.trim().strip_prefix("ref: ").unwrap();
            let branch_name = branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref);
            Ok(branch_name.to_string())
        } else {
            Ok("(detached HEAD)".to_string())
        }
    }

    pub fn hash_object(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    pub fn write_object(&self, content: &[u8]) -> io::Result<String> {
        let hash = self.hash_object(content);
        let objects_dir = if self.is_bare { "objects" } else { ".bloc/objects" };
        let object_dir = format!("{}/{}", objects_dir, &hash[..2]);
        fs::create_dir_all(&object_dir)?;
        
        let object_path = format!("{}/{}", object_dir, &hash[2..]);
        fs::write(object_path, content)?;
        
        Ok(hash)
    }

    pub fn read_object(&self, hash: &str) -> io::Result<Vec<u8>> {
        let objects_dir = if self.is_bare { "objects" } else { ".bloc/objects" };
        let object_path = format!("{}/{}/{}", objects_dir, &hash[..2], &hash[2..]);
        fs::read(object_path)
    }

    pub fn get_refs_dir(&self) -> String {
        if self.is_bare {
            "refs".to_string()
        } else {
            ".bloc/refs".to_string()
        }
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        if self.is_bare {
            return false; // Bare repos don't have working directory files
        }

        let path_str = path.to_string_lossy();
        
        // Always ignore .bloc directory and its contents
        if path_str.contains(".bloc") {
            return true;
        }

        // Check .blocignore
        if let Ok(ignore_content) = fs::read_to_string(".blocignore") {
            for line in ignore_content.lines() {
                let pattern = line.trim();
                if pattern.is_empty() || pattern.starts_with('#') {
                    continue;
                }
                
                // Handle directory patterns ending with /
                if pattern.ends_with('/') {
                    let dir_pattern = &pattern[..pattern.len() - 1];
                    if path_str.starts_with(dir_pattern) || 
                       path_str.starts_with(&format!("./{}", dir_pattern)) ||
                       path_str.contains(&format!("/{}", dir_pattern)) {
                        return true;
                    }
                }
                
                // Handle wildcard patterns
                if pattern.contains('*') {
                    if pattern.starts_with('*') && pattern.ends_with('*') {
                        let middle = &pattern[1..pattern.len() - 1];
                        if path_str.contains(middle) {
                            return true;
                        }
                    } else if pattern.starts_with('*') {
                        let suffix = &pattern[1..];
                        if path_str.ends_with(suffix) {
                            return true;
                        }
                    } else if pattern.ends_with('*') {
                        let prefix = &pattern[..pattern.len() - 1];
                        if path_str.starts_with(prefix) {
                            return true;
                        }
                    }
                }
                
                // Exact match
                if path_str.contains(pattern) {
                    return true;
                }
            }
        }
        
        false
    }

    pub fn get_author_signature(&self) -> String {
        format!("{} <{}>", self.config.user.name, self.config.user.email)
    }
}
