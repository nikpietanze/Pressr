use std::collections::HashMap;
use std::path::Path;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, instrument};
use tokio::fs;

use crate::error::{Error, Result};

/// Request data structure for load testing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestData {
    /// HTTP request body (for POST, PUT, PATCH)
    #[serde(default)]
    pub body: Option<Value>,
    
    /// HTTP request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    /// URL query parameters
    #[serde(default)]
    pub params: HashMap<String, String>,
    
    /// URL path variables
    #[serde(default)]
    pub path_variables: HashMap<String, String>,
    
    /// Variable sets for templating/randomization
    #[serde(default)]
    pub variables: HashMap<String, Vec<String>>,
}

impl RequestData {
    /// Load request data from a JSON file
    #[instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub async fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        debug!("Loading request data from file: {}", path_ref.display());
        
        let content = fs::read_to_string(path_ref).await
            .map_err(|e| Error::DataLoad {
                path: path_ref.to_path_buf(),
                source: Box::new(e),
            })?;
        
        debug!("Parsing JSON data");
        let data: RequestData = serde_json::from_str(&content)
            .map_err(|e| Error::DataLoad {
                path: path_ref.to_path_buf(),
                source: Box::new(e),
            })?;
        
        debug!("Successfully loaded request data");
        Ok(data)
    }
    
    /// Get a random value from a variable set
    pub fn get_random_variable(&self, name: &str) -> Option<&str> {
        self.variables.get(name)
            .and_then(|values| {
                if values.is_empty() {
                    None
                } else {
                    let mut rng = rand::thread_rng();
                    values.choose(&mut rng).map(|s| s.as_str())
                }
            })
    }
} 