use crate::repository::BlocRepo;
use crate::objects::{Commit, IndexEntry};
use std::fs;
use std::io;
use std::path::Path;
use walkdir::WalkDir;
use colored::*;
use chrono::Utc;
use sha2::{Digest, Sha256};

pub fn add_files(repo: &mut BlocRepo, files: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if repo.is_bare {
        println!("{}", "Cannot add files to a bare repository".bright_red().bold());
        return Ok(());
    }

    for pattern in files {
        if pattern == "." {
            // Add all files recursively
            for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() && !repo.should_ignore(path) {
                    add_single_file(repo, path)?;
                }
            }
        } else {
            let path = Path::new(pattern);
            if path.is_file() {
                if !repo.should_ignore(path) {
                    add_single_file(repo, path)?;
                }
            } else if path.is_dir() {
                for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                    let file_path = entry.path();
                    if file_path.is_file() && !repo.should_ignore(file_path) {
                        add_single_file(repo, file_path)?;
                    }
                }
            } else {
                println!("{}: {} {}", 
                        "Warning".bright_yellow().bold(), 
                        pattern.white(), 
                        "does not exist".bright_yellow());
            }
        }
    }
    
    repo.index.save()?;
    Ok(())
}

fn add_single_file(repo: &mut BlocRepo, path: &Path) -> io::Result<()> {
    let content = fs::read_to_string(path)?;
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    
    let relative_path = if let Ok(rel_path) = path.strip_prefix(".") {
        rel_path.to_string_lossy().to_string()
    } else {
        path.to_string_lossy().to_string()
    };
    
    // Store the content as an object
    let objects_dir = repo.bloc_dir.join("objects");
    let object_dir = objects_dir.join(&hash[..2]);
    fs::create_dir_all(&object_dir)?;
    
    let object_path = object_dir.join(&hash[2..]);
    fs::write(&object_path, content.as_bytes())?;
    
    // Add to index
    let entry = IndexEntry {
        hash: hash,
        size: content.len() as u64,
        mode: "100644".to_string(), // Regular file
        mtime: Utc::now(),
    };
    
    repo.index.entries.insert(relative_path.clone(), entry);
    println!("{} {}", "Added".bright_green().bold(), relative_path.bright_cyan());
    
    Ok(())
}

pub fn reset_files(repo: &mut BlocRepo, files: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if repo.is_bare {
        println!("{}", "Cannot reset files in a bare repository".bright_red().bold());
        return Ok(());
    }

    for file in files {
        if repo.index.entries.remove(file).is_some() {
            println!("{} {}", "Reset".bright_yellow().bold(), file.bright_cyan());
        } else {
            println!("{}: {} {}", 
                    "Warning".bright_yellow().bold(), 
                    file.bright_cyan(), 
                    "not in staging area".bright_yellow());
        }
    }
    
    repo.index.save()?;
    Ok(())
}

pub fn commit(repo: &mut BlocRepo, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    if repo.index.entries.is_empty() {
        println!("{}", "Nothing to commit (no files in staging area)".bright_yellow());
        return Ok(());
    }
    
    // Get current head
    let head_path = repo.bloc_dir.join("refs").join("heads").join(&repo.get_current_branch()?);
    let parent = if head_path.exists() {
        Some(fs::read_to_string(&head_path)?.trim().to_string())
    } else {
        None
    };
    
    // Create commit object
    let commit = Commit {
        message: message.to_string(),
        author: repo.config.user.name.clone(),
        committer: repo.config.user.email.clone(),
        timestamp: Utc::now(),
        parent,
        tree: serialize_tree(&repo.index)?,
    };
    
    // Serialize and hash the commit
    let commit_json = serde_json::to_string_pretty(&commit)?;
    let mut hasher = Sha256::new();
    hasher.update(commit_json.as_bytes());
    let commit_hash = format!("{:x}", hasher.finalize());
    
    // Store commit object
    let objects_dir = repo.bloc_dir.join("objects");
    let commit_dir = objects_dir.join(&commit_hash[..2]);
    fs::create_dir_all(&commit_dir)?;
    let commit_path = commit_dir.join(&commit_hash[2..]);
    fs::write(&commit_path, commit_json.as_bytes())?;
    
    // Update HEAD
    fs::write(&head_path, &commit_hash)?;
    
    // Clear the index
    repo.index.entries.clear();
    repo.index.save()?;
    
    println!("{} {} {}", 
             "Committed".bright_green().bold(), 
             &commit_hash[..8].bright_yellow(), 
             message.white());
    
    Ok(())
}

fn serialize_tree(index: &crate::objects::Index) -> Result<String, Box<dyn std::error::Error>> {
    let mut tree_entries = Vec::new();
    
    for (path, entry) in &index.entries {
        tree_entries.push(format!("{}:{}", path, entry.hash));
    }
    
    Ok(tree_entries.join("\n"))
}

pub fn log(repo: &BlocRepo, oneline: bool) -> Result<(), Box<dyn std::error::Error>> {
    let current_branch = repo.get_current_branch()?;
    let head_path = repo.bloc_dir.join("refs").join("heads").join(&current_branch);
    
    if !head_path.exists() {
        println!("{}", "No commits yet".bright_yellow());
        return Ok(());
    }
    
    let mut commit_hash = fs::read_to_string(&head_path)?.trim().to_string();
    
    loop {
        // Read commit object
        let objects_dir = repo.bloc_dir.join("objects");
        let commit_dir = objects_dir.join(&commit_hash[..2]);
        let commit_path = commit_dir.join(&commit_hash[2..]);
        
        if !commit_path.exists() {
            break;
        }
        
        let commit_json = fs::read_to_string(&commit_path)?;
        let commit: Commit = serde_json::from_str(&commit_json)?;
        
        if oneline {
            println!("{} {}", 
                    commit_hash[..8].bright_yellow(), 
                    commit.message.white());
        } else {
            println!("{} {}", "commit".bright_yellow().bold(), commit_hash.bright_yellow());
            println!("{}: {} <{}>", "Author".bright_blue(), commit.author.white(), commit.committer.white());
            println!("{}: {}", "Date".bright_blue(), commit.timestamp.format("%a %b %d %H:%M:%S %Y %z").to_string().white());
            println!();
            println!("    {}", commit.message.white());
            println!();
        }
        
        // Move to parent commit
        if let Some(parent) = commit.parent {
            commit_hash = parent;
        } else {
            break;
        }
    }
    
    Ok(())
}

pub fn status(repo: &BlocRepo) -> Result<(), Box<dyn std::error::Error>> {
    let current_branch = repo.get_current_branch()?;
    println!("{} {}", "On branch".bright_blue(), current_branch.bright_cyan().bold());
    
    if repo.index.entries.is_empty() {
        println!("{}", "No changes staged for commit".bright_green());
    } else {
        println!("{}", "Changes to be committed:".bright_green().bold());
        for (path, _) in &repo.index.entries {
            println!("  {}: {}", "new file".bright_green(), path.white());
        }
    }
    
    // Check for untracked files
    let mut untracked = Vec::new();
    
    if !repo.is_bare {
        for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && !repo.should_ignore(path) {
                let relative_path = if let Ok(rel_path) = path.strip_prefix(".") {
                    rel_path.to_string_lossy().to_string()
                } else {
                    path.to_string_lossy().to_string()
                };
                
                if !repo.index.entries.contains_key(&relative_path) {
                    untracked.push(relative_path);
                }
            }
        }
    }
    
    if !untracked.is_empty() {
        println!();
        println!("{}", "Untracked files:".bright_red().bold());
        println!("  (use \"bloc add <file>...\" to include in what will be committed)");
        println!();
        for file in untracked {
            println!("  {}", file.bright_red());
        }
    }
    
    Ok(())
}
