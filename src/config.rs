use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use colored::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlocConfig {
    pub user: UserConfig,
    pub remotes: HashMap<String, RemoteConfig>,
    pub core: CoreConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfig {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteConfig {
    pub url: String,
    pub fetch: String,
    pub push: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoreConfig {
    pub bare: bool,
    pub default_branch: String,
}

impl Default for BlocConfig {
    fn default() -> Self {
        BlocConfig {
            user: UserConfig {
                name: "Bloc User".to_string(),
                email: "user@bloc.local".to_string(),
            },
            remotes: HashMap::new(),
            core: CoreConfig {
                bare: false,
                default_branch: "main".to_string(),
            },
        }
    }
}

impl BlocConfig {
    pub fn load() -> io::Result<Self> {
        let config_path = if Path::new(".bloc").exists() {
            ".bloc/config"
        } else {
            // Global config in user's home directory
            return Ok(Self::default());
        };

        if Path::new(config_path).exists() {
            let content = fs::read_to_string(config_path)?;
            let config: BlocConfig = serde_json::from_str(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let config_path = ".bloc/config";
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn set_user(&mut self, name: Option<String>, email: Option<String>) -> io::Result<()> {
        if let Some(name) = name {
            self.user.name = name;
        }
        if let Some(email) = email {
            self.user.email = email;
        }
        self.save()?;
        Ok(())
    }

    pub fn add_remote(&mut self, name: String, url: String) -> io::Result<()> {
        let remote = RemoteConfig {
            url: url.clone(),
            fetch: format!("+refs/heads/*:refs/remotes/{}/*", name),
            push: None,
        };
        self.remotes.insert(name.clone(), remote);
        self.save()?;
        println!("{} '{}' -> {}", 
                "Added remote".bright_green().bold(), 
                name.bright_cyan(), 
                url.white());
        Ok(())
    }

    pub fn remove_remote(&mut self, name: &str) -> io::Result<()> {
        if self.remotes.remove(name).is_some() {
            self.save()?;
            println!("{} '{}'", 
                    "Removed remote".bright_yellow().bold(), 
                    name.bright_cyan());
        } else {
            println!("{}: Remote '{}' {}", 
                    "Error".bright_red().bold(), 
                    name.bright_cyan(), 
                    "not found".bright_red());
        }
        Ok(())
    }

    pub fn list_remotes(&self) {
        if self.remotes.is_empty() {
            println!("{}", "No remotes configured".bright_yellow());
        } else {
            for (name, remote) in &self.remotes {
                println!("{}\t{}", name.bright_cyan().bold(), remote.url.white());
            }
        }
    }

    pub fn show_config(&self) {
        println!("{}:", "User Configuration".bright_green().bold());
        println!("  {}: {}", "name".bright_blue(), self.user.name.white());
        println!("  {}: {}", "email".bright_blue(), self.user.email.white());
        
        println!("\n{}:", "Core Configuration".bright_green().bold());
        println!("  {}: {}", "bare".bright_blue(), self.core.bare.to_string().white());
        println!("  {}: {}", "default_branch".bright_blue(), self.core.default_branch.white());
        
        if !self.remotes.is_empty() {
            println!("\n{}:", "Remotes".bright_green().bold());
            for (name, remote) in &self.remotes {
                println!("  {}: {}", name.bright_cyan(), remote.url.white());
            }
        }
    }
}
