use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, error, info, instrument};

use crate::error::{DataError, Result};

/// Represents a data file containing request information for load testing
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RequestData {
    /// Optional request body to use for POST/PUT/PATCH methods
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    
    /// Optional headers for the requests
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    /// Optional URL parameters
    #[serde(default)]
    pub params: HashMap<String, String>,
    
    /// Optional path variables
    #[serde(default)]
    pub path_variables: HashMap<String, String>,
    
    /// List of variable data to use for requests (randomly selected during testing)
    #[serde(default)]
    pub variables: Vec<HashMap<String, serde_json::Value>>,
}

impl Default for RequestData {
    fn default() -> Self {
        Self {
            body: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            path_variables: HashMap::new(),
            variables: Vec::new(),
        }
    }
}

impl RequestData {
    /// Load request data from a JSON file
    #[instrument(skip(path), err)]
    pub async fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let path_str = path_ref.display().to_string();
        
        debug!("Loading data file from {}", path_str);
        
        // Check if file exists
        if !path_ref.exists() {
            error!("File not found: {}", path_str);
            return Err(DataError::FileNotFound(path_str).into());
        }
        
        // Read file content
        debug!("Reading file content");
        let content = fs::read_to_string(path_ref).await
            .map_err(|e| {
                error!("Failed to read file: {}", e);
                DataError::FileRead(e)
            })?;
        
        // Parse JSON
        debug!("Parsing JSON content");
        let data = serde_json::from_str(&content)
            .map_err(|e| {
                error!("Failed to parse JSON: {}", e);
                DataError::JsonParse(e)
            })?;
        
        info!("Successfully loaded data file");
        Ok(data)
    }
    
    /// Get a random variable set from the variables list
    /// Returns None if variables list is empty
    #[instrument(skip(self))]
    pub fn get_random_variables(&self) -> Option<&HashMap<String, serde_json::Value>> {
        if self.variables.is_empty() {
            debug!("Variables list is empty");
            return None;
        }
        
        let index = rand::random::<usize>() % self.variables.len();
        debug!("Selected variable set at index {}", index);
        self.variables.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_request_data_default() {
        let data = RequestData::default();
        assert!(data.body.is_none());
        assert!(data.headers.is_empty());
        assert!(data.params.is_empty());
        assert!(data.path_variables.is_empty());
        assert!(data.variables.is_empty());
    }
    
    // Other tests would go here
} 