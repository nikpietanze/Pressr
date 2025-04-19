use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::Duration;

/// Result of a single HTTP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestResult {
    /// HTTP status code
    pub status: Option<u16>,
    
    /// Response time in milliseconds
    pub response_time: u128,
    
    /// Whether the request was successful
    pub success: bool,
    
    /// Error message, if any
    pub error: Option<String>,
    
    /// Response size in bytes
    pub response_size: Option<usize>,
}

/// Results of a load test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResults {
    /// Total number of requests sent
    pub total_requests: usize,
    
    /// Number of successful requests
    pub successful_requests: usize,
    
    /// Number of failed requests
    pub failed_requests: usize,
    
    /// Average response time in milliseconds
    pub average_response_time: f64,
    
    /// Minimum response time in milliseconds
    pub min_response_time: u128,
    
    /// Maximum response time in milliseconds
    pub max_response_time: u128,
    
    /// Total test duration
    #[serde(skip)]
    pub duration: Duration,
    
    /// Test duration in seconds (serializable)
    #[serde(rename = "duration")]
    pub duration_secs: f64,
    
    /// Status code distribution
    pub status_codes: HashMap<u16, usize>,
    
    /// Error message distribution
    pub errors: HashMap<String, usize>,
    
    /// Individual request results
    pub requests: Vec<RequestResult>,
}

impl LoadTestResults {
    /// Create a new LoadTestResults
    pub fn new(requests: Vec<RequestResult>, duration: Duration) -> Self {
        let total_requests = requests.len();
        let successful_requests = requests.iter().filter(|r| r.success).count();
        let failed_requests = total_requests - successful_requests;
        
        // Calculate response time statistics
        let mut min_response_time = u128::MAX;
        let mut max_response_time = 0;
        let mut total_response_time = 0;
        
        // Build status code and error distributions
        let mut status_codes = HashMap::new();
        let mut errors = HashMap::new();
        
        for result in &requests {
            // Response time stats
            min_response_time = min_response_time.min(result.response_time);
            max_response_time = max_response_time.max(result.response_time);
            total_response_time += result.response_time;
            
            // Status code distribution
            if let Some(status) = result.status {
                *status_codes.entry(status).or_insert(0) += 1;
            }
            
            // Error distribution
            if let Some(error) = &result.error {
                *errors.entry(error.clone()).or_insert(0) += 1;
            }
        }
        
        // Handle edge case of empty results
        if total_requests == 0 {
            min_response_time = 0;
        }
        
        let average_response_time = if total_requests > 0 {
            total_response_time as f64 / total_requests as f64
        } else {
            0.0
        };
        
        Self {
            total_requests,
            successful_requests,
            failed_requests,
            average_response_time,
            min_response_time,
            max_response_time,
            duration,
            duration_secs: duration.as_secs_f64(),
            status_codes,
            errors,
            requests,
        }
    }
} 