//------------------------------------------------------------------------------------------------------------
// file: config.rs
// purpose: Multi-tenant configuration loading for SQ v0.5.5
//------------------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TenantConfig {
    pub name: String,
    pub data_dir: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub tenants: HashMap<String, TenantConfig>, // key = API token (e.g., "pmb-v1-xxx")
}

/// Load multi-tenant configuration from JSON file
/// Automatically creates tenant data directories if they don't exist
pub fn load_config(path: &str) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: ServerConfig = serde_json::from_str(&content)?;
    
    // Auto-create tenant data directories
    for (token, tenant) in &config.tenants {
        if let Err(e) = std::fs::create_dir_all(&tenant.data_dir) {
            eprintln!("Warning: Failed to create data directory for tenant {} ({}): {}", 
                      tenant.name, token, e);
            // Continue anyway - directory might already exist or be created later
        }
    }
    
    Ok(config)
}
