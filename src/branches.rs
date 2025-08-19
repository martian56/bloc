use crate::repository::BlocRepo;
use std::fs;
use std::io;
// use std::path::Path;
use colored::*;

pub fn create_branch(repo: &mut BlocRepo, name: &str) -> io::Result<()> {
    let refs_dir = repo.bloc_dir.join("refs");
    let branch_ref_path = refs_dir.join("heads").join(name);
    
    if branch_ref_path.exists() {
        println!("{} '{}' {}", 
                "Branch".bright_yellow(), 
                name.bright_cyan(), 
                "already exists".bright_yellow());
        return Ok(());
    }

    // Get current commit hash
    if let Ok(current_hash) = get_current_commit_hash(repo) {
        fs::write(branch_ref_path, current_hash)?;
        println!("{} '{}'", 
                "Created branch".bright_green().bold(), 
                name.bright_cyan().bold());
    } else {
        println!("{}: {}", 
                "Cannot create branch".bright_red().bold(), 
                "no commits yet".bright_red());
    }
    
    Ok(())
}

pub fn delete_branch(repo: &mut BlocRepo, name: &str, force: bool) -> io::Result<()> {
    let current_branch = repo.get_current_branch().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    if current_branch == name {
        println!("{}: {}", 
                "Cannot delete branch".bright_red().bold(), 
                "currently checked out".bright_red());
        return Ok(());
    }

    let refs_dir = repo.bloc_dir.join("refs");
    let branch_ref_path = refs_dir.join("heads").join(name);
    
    if !branch_ref_path.exists() {
        println!("{} '{}' {}", 
                "Branch".bright_yellow(), 
                name.bright_cyan(), 
                "does not exist".bright_yellow());
        return Ok(());
    }

    if !force {
        // TODO: Check if branch is merged
        println!("{}: {} {}", 
                "Use --force to delete".bright_yellow().bold(), 
                name.bright_cyan(), 
                "(branch merge check not implemented)".bright_yellow());
        return Ok(());
    }

    fs::remove_file(branch_ref_path)?;
    println!("{} '{}'", 
            "Deleted branch".bright_red().bold(), 
            name.bright_cyan());
    
    Ok(())
}

pub fn list_branches(repo: &BlocRepo) -> io::Result<()> {
    let refs_dir = repo.bloc_dir.join("refs").join("heads");
    
    if !refs_dir.exists() {
        println!("{}", "No branches found".bright_yellow());
        return Ok(());
    }

    let current_branch = repo.get_current_branch().unwrap_or_else(|_| "master".to_string());
    
    for entry in fs::read_dir(refs_dir)? {
        let entry = entry?;
        let branch_name = entry.file_name().to_string_lossy().to_string();
        
        if branch_name == current_branch {
            println!("{} {}", "*".bright_green().bold(), branch_name.bright_green().bold());
        } else {
            println!("  {}", branch_name.white());
        }
    }
    
    Ok(())
}

pub fn checkout(repo: &mut BlocRepo, branch_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let refs_dir = repo.bloc_dir.join("refs").join("heads");
    let branch_ref_path = refs_dir.join(branch_name);
    
    if !branch_ref_path.exists() {
        println!("{} '{}' {}", 
                "Branch".bright_red().bold(), 
                branch_name.bright_cyan(), 
                "does not exist".bright_red());
        return Ok(());
    }

    // Update HEAD to point to the new branch
    let head_path = repo.bloc_dir.join("HEAD");
    let head_content = format!("ref: refs/heads/{}", branch_name);
    fs::write(head_path, head_content)?;
    
    println!("{} '{}'", 
            "Switched to branch".bright_green().bold(), 
            branch_name.bright_cyan().bold());
    
    Ok(())
}

pub fn rename_branch(repo: &mut BlocRepo, old_name: &str, new_name: &str) -> io::Result<()> {
    let refs_dir = repo.bloc_dir.join("refs").join("heads");
    let old_path = refs_dir.join(old_name);
    let new_path = refs_dir.join(new_name);
    
    if !old_path.exists() {
        println!("{} '{}' {}", 
                "Branch".bright_red().bold(), 
                old_name.bright_cyan(), 
                "does not exist".bright_red());
        return Ok(());
    }

    if new_path.exists() {
        println!("{} '{}' {}", 
                "Branch".bright_red().bold(), 
                new_name.bright_cyan(), 
                "already exists".bright_red());
        return Ok(());
    }

    fs::rename(old_path, new_path)?;
    
    // Update HEAD if it was pointing to the renamed branch
    let head_path = repo.bloc_dir.join("HEAD");
    if let Ok(head_content) = fs::read_to_string(&head_path) {
        if head_content.trim() == format!("ref: refs/heads/{}", old_name) {
            let new_head_content = format!("ref: refs/heads/{}", new_name);
            fs::write(head_path, new_head_content)?;
        }
    }
    
    println!("{} '{}' {} '{}'", 
            "Renamed branch".bright_green().bold(), 
            old_name.bright_cyan(), 
            "to".bright_green(), 
            new_name.bright_cyan().bold());
    
    Ok(())
}

fn get_current_commit_hash(repo: &BlocRepo) -> Result<String, Box<dyn std::error::Error>> {
    let current_branch = repo.get_current_branch()?;
    let refs_dir = repo.bloc_dir.join("refs").join("heads");
    let branch_ref_path = refs_dir.join(current_branch);
    
    if branch_ref_path.exists() {
        Ok(fs::read_to_string(branch_ref_path)?.trim().to_string())
    } else {
        Err("No commits found".into())
    }
}
