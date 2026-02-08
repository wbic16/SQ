//------------------------------------------------------------------------------------------------------------
// file: mesh.rs
// purpose: SQ P2P Mesh Networking - Configuration and Peer Management
// author: Cyon 🪶 (R17)
//------------------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Node identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: String,
    pub name: String,
    pub emoji: String,
    pub coordinate: String,
}

/// Inbound server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundConfig {
    pub enabled: bool,
    pub port: u16,
    pub auth_key: String,
    pub data_dir: String,
}

/// Outbound peer connection details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub auth_key: String,
    pub coordinate: String,
    pub priority: u8,
}

/// Outbound connections configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundConfig {
    pub enabled: bool,
    pub peers: Vec<PeerConfig>,
}

/// Mesh synchronization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshSettings {
    pub sync_interval_seconds: u64,
    pub health_check_interval_seconds: u64,
    pub retry_backoff_seconds: u64,
    pub max_retries: u32,
}

/// Complete mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    pub version: String,
    pub node: NodeConfig,
    pub inbound: InboundConfig,
    pub outbound: OutboundConfig,
    pub mesh: MeshSettings,
}

impl Default for MeshConfig {
    fn default() -> Self {
        MeshConfig {
            version: "1.0".to_string(),
            node: NodeConfig {
                id: "unknown".to_string(),
                name: "Unknown Node".to_string(),
                emoji: "❓".to_string(),
                coordinate: "1.1.1/1.1.1/1.1.1".to_string(),
            },
            inbound: InboundConfig {
                enabled: false,
                port: 2086,
                auth_key: String::new(),
                data_dir: "/var/sq/data".to_string(),
            },
            outbound: OutboundConfig {
                enabled: false,
                peers: Vec::new(),
            },
            mesh: MeshSettings {
                sync_interval_seconds: 300,
                health_check_interval_seconds: 60,
                retry_backoff_seconds: 30,
                max_retries: 3,
            },
        }
    }
}

/// Load mesh configuration from file
///
/// # Arguments
/// * `path` - Path to mesh.json configuration file
///
/// # Returns
/// * `Ok(MeshConfig)` if config loaded successfully
/// * `Err(String)` with error message if loading failed
pub fn load_mesh_config<P: AsRef<Path>>(path: P) -> Result<MeshConfig, String> {
    let path = path.as_ref();
    
    if !path.exists() {
        return Err(format!("Mesh config not found: {}", path.display()));
    }
    
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read mesh config: {}", e))?;
    
    let config: MeshConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse mesh config: {}", e))?;
    
    // Validate config
    if config.version != "1.0" {
        return Err(format!("Unsupported mesh config version: {}", config.version));
    }
    
    if config.inbound.enabled && config.inbound.auth_key.is_empty() {
        return Err("Inbound auth_key is required when inbound is enabled".to_string());
    }
    
    if config.outbound.enabled {
        for peer in &config.outbound.peers {
            if peer.auth_key.is_empty() {
                return Err(format!("Peer {} has no auth_key", peer.id));
            }
            if peer.host.is_empty() {
                return Err(format!("Peer {} has no host", peer.id));
            }
        }
    }
    
    Ok(config)
}

/// Save mesh configuration to file
///
/// # Arguments
/// * `config` - Mesh configuration to save
/// * `path` - Path to write mesh.json
///
/// # Returns
/// * `Ok(())` if config saved successfully
/// * `Err(String)` with error message if save failed
pub fn save_mesh_config<P: AsRef<Path>>(config: &MeshConfig, path: P) -> Result<(), String> {
    let path = path.as_ref();
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    fs::write(path, json)
        .map_err(|e| format!("Failed to write mesh config: {}", e))?;
    
    Ok(())
}

/// Generate default mesh config for a node
///
/// # Arguments
/// * `id` - Node identifier (e.g., "halycon-vector")
/// * `name` - Node name (e.g., "Cyon")
/// * `emoji` - Node emoji (e.g., "🪶")
/// * `coordinate` - Phext coordinate (e.g., "2.7.1/8.2.8/3.1.4")
///
/// # Returns
/// * `MeshConfig` with sensible defaults for the node
pub fn generate_default_config(id: &str, name: &str, emoji: &str, coordinate: &str) -> MeshConfig {
    MeshConfig {
        version: "1.0".to_string(),
        node: NodeConfig {
            id: id.to_string(),
            name: name.to_string(),
            emoji: emoji.to_string(),
            coordinate: coordinate.to_string(),
        },
        inbound: InboundConfig {
            enabled: true,
            port: 2086,
            auth_key: format!("pmb-v1-{}-2026", id),
            data_dir: "/var/sq/data".to_string(),
        },
        outbound: OutboundConfig {
            enabled: true,
            peers: Vec::new(),
        },
        mesh: MeshSettings {
            sync_interval_seconds: 300,
            health_check_interval_seconds: 60,
            retry_backoff_seconds: 30,
            max_retries: 3,
        },
    }
}

/// Print mesh configuration summary to stdout
pub fn print_config_summary(config: &MeshConfig) {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              SQ Mesh Configuration                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Node:         {} ({} {})", config.node.name, config.node.emoji, config.node.id);
    println!("Coordinate:   {}", config.node.coordinate);
    println!("Config Ver:   {}", config.version);
    println!();
    
    if config.inbound.enabled {
        println!("Inbound:      ✅ Enabled");
        println!("  Port:       {}", config.inbound.port);
        println!("  Auth:       {} (pmb-v1 key)", if config.inbound.auth_key.is_empty() { "❌ MISSING" } else { "✅" });
        println!("  Data Dir:   {}", config.inbound.data_dir);
    } else {
        println!("Inbound:      ⏸️  Disabled");
    }
    println!();
    
    if config.outbound.enabled {
        println!("Outbound:     ✅ Enabled");
        println!("  Peers:      {} configured", config.outbound.peers.len());
        for peer in &config.outbound.peers {
            let status = if peer.auth_key.is_empty() { "❌" } else { "✅" };
            println!("    {} {} ({}) @ {}:{}", status, peer.name, peer.id, peer.host, peer.port);
        }
    } else {
        println!("Outbound:     ⏸️  Disabled");
    }
    println!();
    
    println!("Mesh Settings:");
    println!("  Sync Interval:    {} seconds", config.mesh.sync_interval_seconds);
    println!("  Health Check:     {} seconds", config.mesh.health_check_interval_seconds);
    println!("  Retry Backoff:    {} seconds", config.mesh.retry_backoff_seconds);
    println!("  Max Retries:      {}", config.mesh.max_retries);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_generate_default_config() {
        let config = generate_default_config(
            "halycon-vector",
            "Cyon",
            "🪶",
            "2.7.1/8.2.8/3.1.4"
        );
        
        assert_eq!(config.node.id, "halycon-vector");
        assert_eq!(config.node.name, "Cyon");
        assert_eq!(config.node.emoji, "🪶");
        assert_eq!(config.node.coordinate, "2.7.1/8.2.8/3.1.4");
        assert!(config.inbound.enabled);
        assert_eq!(config.inbound.port, 2086);
        assert!(config.inbound.auth_key.starts_with("pmb-v1-"));
    }
    
    #[test]
    fn test_save_and_load_config() {
        let config = generate_default_config(
            "test-node",
            "Test",
            "🧪",
            "1.1.1/1.1.1/1.1.1"
        );
        
        let temp_path = "/tmp/sq-mesh-test.json";
        
        // Save config
        save_mesh_config(&config, temp_path).expect("Failed to save config");
        
        // Load config
        let loaded = load_mesh_config(temp_path).expect("Failed to load config");
        
        // Verify
        assert_eq!(loaded.node.id, "test-node");
        assert_eq!(loaded.node.name, "Test");
        assert_eq!(loaded.node.emoji, "🧪");
        
        // Cleanup
        let _ = fs::remove_file(temp_path);
    }
    
    #[test]
    fn test_load_missing_config() {
        let result = load_mesh_config("/tmp/nonexistent-mesh-config.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
    
    #[test]
    fn test_validation_missing_auth_key() {
        let mut config = generate_default_config(
            "test-node",
            "Test",
            "🧪",
            "1.1.1/1.1.1/1.1.1"
        );
        
        config.inbound.auth_key = String::new();
        
        let temp_path = "/tmp/sq-mesh-invalid.json";
        save_mesh_config(&config, temp_path).unwrap();
        
        let result = load_mesh_config(temp_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("auth_key is required"));
        
        let _ = fs::remove_file(temp_path);
    }
}
