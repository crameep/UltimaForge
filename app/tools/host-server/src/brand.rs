//! Brand configuration loading for host-server startup branding.

use serde::Deserialize;
use std::path::Path;

/// Product information from brand.json
#[derive(Debug, Deserialize)]
pub struct ProductInfo {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub description: Option<String>,
}

/// Minimal brand configuration for displaying server name
#[derive(Debug, Deserialize)]
pub struct BrandConfig {
    pub product: ProductInfo,
}

impl BrandConfig {
    /// Loads brand configuration from branding/brand.json
    pub fn load() -> Option<Self> {
        let brand_file = "branding/brand.json";

        // Search in multiple locations
        let search_paths = vec![
            // Relative to CWD (when running from project root)
            std::path::PathBuf::from(brand_file),
            // One level up (when running from tools/host-server)
            std::path::PathBuf::from("../../branding/brand.json"),
            // Two levels up (when running from target/debug)
            std::path::PathBuf::from("../../../branding/brand.json"),
        ];

        for path in search_paths {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(config) = serde_json::from_str::<BrandConfig>(&contents) {
                        return Some(config);
                    }
                }
            }
        }

        None
    }
}
