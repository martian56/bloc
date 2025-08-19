mod config;
mod repository;
mod objects;
mod commands;
mod branches;

use clap::{Parser, Subcommand};
use repository::BlocRepo;
use config::BlocConfig;
use std::io;
use colored::*;

#[cfg(windows)]
fn hide_directory(path: &str) -> io::Result<()> {
    use std::process::Command;
    
    // Use Windows attrib command to set hidden attribute
    let output = Command::new("attrib")
        .args(&["+H", path])
        .output()?;
    
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to hide directory: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
}

#[cfg(not(windows))]
fn hide_directory(_path: &str) -> io::Result<()> {
    // On Unix-like systems, directories starting with . are hidden by default
    Ok(())
}

#[derive(Parser)]
#[command(name = "bloc")]
#[command(about = "A powerful git-like version control tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new bloc repository
    Init {
        /// Repository path (optional)
        path: Option<String>,
        /// Create a bare repository
        #[arg(long)]
        bare: bool,
    },
    /// Clone a repository
    Clone {
        /// Repository URL or path
        url: String,
        /// Destination directory (optional)
        directory: Option<String>,
    },
    /// Add file(s) to the staging area
    Add {
        files: Vec<String>,
    },
    /// Remove files from the staging area
    Reset {
        files: Vec<String>,
    },
    /// Commit staged changes
    Commit {
        #[arg(short, long)]
        message: String,
    },
    /// Show commit log
    Log {
        #[arg(short, long)]
        oneline: bool,
    },
    /// Show repository status
    Status,
    /// Show differences
    Diff {
        /// Show staged changes
        #[arg(long)]
        staged: bool,
    },
    /// Branch operations
    Branch {
        /// Branch name to create
        name: Option<String>,
        /// List all branches
        #[arg(short, long)]
        list: bool,
        /// Delete a branch
        #[arg(short, long)]
        delete: Option<String>,
        /// Force delete
        #[arg(long)]
        force: bool,
        /// Rename a branch
        #[arg(short, long)]
        rename: Option<Vec<String>>,
    },
    /// Switch to a different branch
    Checkout {
        branch: String,
    },
    /// Merge a branch into current branch
    Merge {
        branch: String,
    },
    /// Show file contents at specific commit
    Show {
        #[arg(help = "commit-hash:path or just commit-hash")]
        target: String,
    },
    /// Remove files from working directory and index
    Rm {
        files: Vec<String>,
        /// Remove directories recursively
        #[arg(short)]
        recursive: bool,
    },
    /// Configuration operations
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Remote repository operations
    Remote {
        #[command(subcommand)]
        action: RemoteCommands,
    },
    /// Push changes to remote repository
    Push {
        /// Remote name (default: origin)
        remote: Option<String>,
        /// Branch name (default: current branch)
        branch: Option<String>,
        /// Force push
        #[arg(long)]
        force: bool,
    },
    /// Fetch changes from remote repository
    Fetch {
        /// Remote name (default: origin)
        remote: Option<String>,
    },
    /// Pull changes from remote repository
    Pull {
        /// Remote name (default: origin)
        remote: Option<String>,
        /// Branch name (default: current branch)
        branch: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set configuration value
    Set {
        /// Configuration key (e.g., user.name, user.email)
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// List all configuration
    List,
}

#[derive(Subcommand)]
enum RemoteCommands {
    /// Add a remote repository
    Add {
        /// Remote name
        name: String,
        /// Remote URL
        url: String,
    },
    /// Remove a remote repository
    Remove {
        /// Remote name
        name: String,
    },
    /// List remote repositories
    List,
    /// Show remote repository details
    Show {
        /// Remote name
        name: String,
    },
    /// Rename a remote repository
    Rename {
        /// Old name
        old_name: String,
        /// New name
        new_name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { path, bare } => {
            match BlocRepo::init(path.as_deref(), *bare) {
                Ok(_) => {},
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Clone { url, directory } => {
            println!("{}: {}", 
                    "Clone functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("URL: {}", url.bright_cyan());
            if let Some(dir) = directory {
                println!("Directory: {}", dir.bright_cyan());
            }
        }

        Commands::Config { action } => {
            handle_config_command(action);
        }

        Commands::Remote { action } => {
            handle_remote_command(action);
        }

        Commands::Add { files } => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(mut repo) => {
                    if let Err(e) = commands::add_files(&mut repo, files) {
                        println!("{}: {}", "Error adding files".bright_red().bold(), e);
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Reset { files } => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(mut repo) => {
                    if let Err(e) = commands::reset_files(&mut repo, files) {
                        println!("{}: {}", "Error resetting files".bright_red().bold(), e);
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Commit { message } => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(mut repo) => {
                    if let Err(e) = commands::commit(&mut repo, message) {
                        println!("{}: {}", "Error committing".bright_red().bold(), e);
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Log { oneline } => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(repo) => {
                    if let Err(e) = commands::log(&repo, *oneline) {
                        println!("{}: {}", "Error showing log".bright_red().bold(), e);
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Status => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(repo) => {
                    if let Err(e) = commands::status(&repo) {
                        println!("{}: {}", "Error showing status".bright_red().bold(), e);
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Branch { name, list, delete, force, rename } => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(mut repo) => {
                    if let Some(branch_to_delete) = delete {
                        if let Err(e) = branches::delete_branch(&mut repo, branch_to_delete, *force) {
                            println!("{}: {}", "Error deleting branch".bright_red().bold(), e);
                        }
                    } else if let Some(rename_args) = rename {
                        if rename_args.len() == 2 {
                            if let Err(e) = branches::rename_branch(&mut repo, &rename_args[0], &rename_args[1]) {
                                println!("{}: {}", "Error renaming branch".bright_red().bold(), e);
                            }
                        } else {
                            println!("{}: {}", 
                                    "Error".bright_red().bold(), 
                                    "Rename requires old and new branch names".bright_red());
                        }
                    } else if *list || name.is_none() {
                        if let Err(e) = branches::list_branches(&repo) {
                            println!("{}: {}", "Error listing branches".bright_red().bold(), e);
                        }
                    } else if let Some(branch_name) = name {
                        if let Err(e) = branches::create_branch(&mut repo, branch_name) {
                            println!("{}: {}", "Error creating branch".bright_red().bold(), e);
                        }
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }
        
        Commands::Checkout { branch } => {
            if !BlocRepo::is_repo() {
                println!("{}: {}. {}", 
                        "Error".bright_red().bold(),
                        "Not a bloc repository".bright_red(), 
                        "Run 'bloc init' first".bright_yellow());
                return;
            }
            
            match BlocRepo::new() {
                Ok(mut repo) => {
                    if let Err(e) = branches::checkout(&mut repo, branch) {
                        println!("{}: {}", "Error checking out branch".bright_red().bold(), e);
                    }
                }
                Err(e) => println!("{}: {}", "Error".bright_red().bold(), e),
            }
        }

        Commands::Push { remote, branch, force } => {
            println!("{}: {}", 
                    "Push functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("Remote: {}", remote.as_deref().unwrap_or("origin").bright_cyan());
            if let Some(b) = branch {
                println!("Branch: {}", b.bright_cyan());
            }
            if *force {
                println!("Force: {}", "true".bright_red());
            }
        }

        Commands::Fetch { remote } => {
            println!("{}: {}", 
                    "Fetch functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("Remote: {}", remote.as_deref().unwrap_or("origin").bright_cyan());
        }

        Commands::Pull { remote, branch } => {
            println!("{}: {}", 
                    "Pull functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("Remote: {}", remote.as_deref().unwrap_or("origin").bright_cyan());
            if let Some(b) = branch {
                println!("Branch: {}", b.bright_cyan());
            }
        }

        Commands::Diff { staged } => {
            println!("{}: {}", 
                    "Diff functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            if *staged {
                println!("Mode: {}", "staged changes".bright_cyan());
            } else {
                println!("Mode: {}", "working directory changes".bright_cyan());
            }
        }

        Commands::Merge { branch } => {
            println!("{}: {}", 
                    "Merge functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("Branch: {}", branch.bright_cyan());
        }

        Commands::Show { target } => {
            println!("{}: {}", 
                    "Show functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("Target: {}", target.bright_cyan());
        }

        Commands::Rm { files, recursive } => {
            println!("{}: {}", 
                    "Remove functionality".bright_yellow().bold(), 
                    "not yet implemented".bright_yellow());
            println!("Files: {}", files.join(", ").bright_cyan());
            if *recursive {
                println!("Recursive: {}", "true".bright_red());
            }
        }
    }
}

fn handle_config_command(action: &ConfigCommands) {
    match BlocConfig::load() {
        Ok(mut config) => {
            match action {
                ConfigCommands::Set { key, value } => {
                    match key.as_str() {
                        "user.name" => {
                            if let Err(e) = config.set_user(Some(value.clone()), None) {
                                println!("{}: {}", "Error".bright_red().bold(), e);
                            } else {
                                println!("{} {} = {}", 
                                        "Set".bright_green().bold(), 
                                        key.bright_blue(), 
                                        value.white());
                            }
                        }
                        "user.email" => {
                            if let Err(e) = config.set_user(None, Some(value.clone())) {
                                println!("{}: {}", "Error".bright_red().bold(), e);
                            } else {
                                println!("{} {} = {}", 
                                        "Set".bright_green().bold(), 
                                        key.bright_blue(), 
                                        value.white());
                            }
                        }
                        _ => {
                            println!("{}: {} {}", 
                                    "Error".bright_red().bold(), 
                                    "Unknown configuration key".bright_red(), 
                                    key.bright_cyan());
                        }
                    }
                }
                ConfigCommands::Get { key } => {
                    match key.as_str() {
                        "user.name" => println!("{}", config.user.name.white()),
                        "user.email" => println!("{}", config.user.email.white()),
                        _ => println!("{}: {}", 
                                    "Error".bright_red().bold(), 
                                    "Unknown configuration key".bright_red()),
                    }
                }
                ConfigCommands::List => {
                    config.show_config();
                }
            }
        }
        Err(e) => println!("{}: {}", "Error loading config".bright_red().bold(), e),
    }
}

fn handle_remote_command(action: &RemoteCommands) {
    if !BlocRepo::is_repo() {
        println!("{}: {}. {}", 
                "Error".bright_red().bold(),
                "Not a bloc repository".bright_red(), 
                "Run 'bloc init' first".bright_yellow());
        return;
    }

    match BlocConfig::load() {
        Ok(mut config) => {
            match action {
                RemoteCommands::Add { name, url } => {
                    if let Err(e) = config.add_remote(name.clone(), url.clone()) {
                        println!("{}: {}", "Error".bright_red().bold(), e);
                    }
                }
                RemoteCommands::Remove { name } => {
                    if let Err(e) = config.remove_remote(name) {
                        println!("{}: {}", "Error".bright_red().bold(), e);
                    }
                }
                RemoteCommands::List => {
                    config.list_remotes();
                }
                RemoteCommands::Show { name } => {
                    if let Some(remote) = config.remotes.get(name) {
                        println!("{}:", name.bright_cyan().bold());
                        println!("  {}: {}", "URL".bright_blue(), remote.url.white());
                        println!("  {}: {}", "Fetch".bright_blue(), remote.fetch.white());
                        if let Some(push) = &remote.push {
                            println!("  {}: {}", "Push".bright_blue(), push.white());
                        }
                    } else {
                        println!("{}: Remote '{}' {}", 
                                "Error".bright_red().bold(), 
                                name.bright_cyan(), 
                                "not found".bright_red());
                    }
                }
                RemoteCommands::Rename { old_name, new_name } => {
                    if let Some(remote) = config.remotes.remove(old_name) {
                        config.remotes.insert(new_name.clone(), remote);
                        if let Err(e) = config.save() {
                            println!("{}: {}", "Error".bright_red().bold(), e);
                        } else {
                            println!("{} '{}' {} '{}'", 
                                    "Renamed remote".bright_green().bold(), 
                                    old_name.bright_cyan(), 
                                    "to".bright_green(), 
                                    new_name.bright_cyan().bold());
                        }
                    } else {
                        println!("{}: Remote '{}' {}", 
                                "Error".bright_red().bold(), 
                                old_name.bright_cyan(), 
                                "not found".bright_red());
                    }
                }
            }
        }
        Err(e) => println!("{}: {}", "Error loading config".bright_red().bold(), e),
    }
}
