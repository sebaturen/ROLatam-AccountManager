use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server_url: String,
    pub temp_key: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "config.json";
        
        if !Path::new(config_path).exists() {
            tracing::warn!("config.json not found, creating default config");
            let default_config = Config {
                server_url: "ws://localhost:8765".to_string(),
                temp_key: Some("CHANGE_ME".to_string()),
            };
            
            let config_json = serde_json::to_string_pretty(&default_config)?;
            fs::write(config_path, config_json)?;
            
            tracing::info!("Default config.json created. Please edit it with your server_url and temp_key");
            return Ok(default_config);
        }

        let config_content = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)?;
        
        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecureStorage {
    pub permanent_key: String,
    pub client_id: String,
}

impl SecureStorage {
    const STORAGE_FILE: &'static str = "client_data.enc";
    
    pub fn load() -> Option<Self> {
        if !Path::new(Self::STORAGE_FILE).exists() {
            return None;
        }

        match fs::read_to_string(Self::STORAGE_FILE) {
            Ok(content) => {
                // Simple encoding for now, can be enhanced with encryption
                match STANDARD.decode(content.trim()) {
                    Ok(decoded) => {
                        match serde_json::from_slice(&decoded) {
                            Ok(storage) => Some(storage),
                            Err(e) => {
                                tracing::error!("Failed to parse secure storage: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode secure storage: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to read secure storage: {}", e);
                None
            }
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(self)?;
        let encoded = STANDARD.encode(json.as_bytes());
        fs::write(Self::STORAGE_FILE, encoded)?;
        Ok(())
    }

    pub fn new(permanent_key: String, client_id: String) -> Self {
        Self {
            permanent_key,
            client_id,
        }
    }
}
