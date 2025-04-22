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
    
    /// Throughput in requests per second
    pub throughput: f64,
    
    /// Total data transferred in bytes (if response sizes are available)
    pub total_data_transferred: Option<usize>,
    
    /// Response time standard deviation in milliseconds
    pub response_time_std_dev: f64,
    
    /// Transfer rate in bytes per second (if response sizes are available)
    pub transfer_rate: Option<f64>,
    
    /// Distribution of response times in buckets (for histograms)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub response_time_distribution: HashMap<String, usize>,
}

impl LoadTestResults {
    /// Create a new LoadTestResults
    pub fn new(requests: Vec<RequestResult>, duration: Duration) -> Self {
        let total_requests = requests.len();
        let successful_requests = requests.iter().filter(|r| r.success).count();
        let failed_requests = total_requests - successful_requests;
        let duration_secs = duration.as_secs_f64();
        
        // Calculate response time statistics
        let mut min_response_time = u128::MAX;
        let mut max_response_time = 0;
        let mut total_response_time = 0;
        let mut sum_squared_diff = 0.0;
        
        // Build status code and error distributions
        let mut status_codes = HashMap::new();
        let mut errors = HashMap::new();
        
        // Calculate total data transferred
        let mut total_data = 0;
        let mut has_all_response_sizes = true;
        
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
            
            // Data transfer stats
            if let Some(size) = result.response_size {
                total_data += size;
            } else {
                has_all_response_sizes = false;
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
        
        // Calculate standard deviation
        for result in &requests {
            let diff = result.response_time as f64 - average_response_time;
            sum_squared_diff += diff * diff;
        }
        
        let response_time_std_dev = if total_requests > 1 {
            (sum_squared_diff / (total_requests as f64 - 1.0)).sqrt()
        } else {
            0.0
        };
        
        // Calculate throughput
        let throughput = if duration_secs > 0.0 {
            total_requests as f64 / duration_secs
        } else {
            0.0
        };
        
        // Create response time distribution for histograms
        let mut response_time_distribution = HashMap::new();
        if !requests.is_empty() {
            // Create buckets for response times
            let bucket_size = if max_response_time > 1000 { 100 } else { 10 };
            for result in &requests {
                let bucket = (result.response_time / bucket_size) * bucket_size;
                let bucket_key = format!("{}-{}", bucket, bucket + bucket_size);
                *response_time_distribution.entry(bucket_key).or_insert(0) += 1;
            }
        }
        
        Self {
            total_requests,
            successful_requests,
            failed_requests,
            average_response_time,
            min_response_time,
            max_response_time,
            duration,
            duration_secs,
            status_codes,
            errors,
            requests,
            throughput,
            total_data_transferred: if has_all_response_sizes { Some(total_data) } else { None },
            response_time_std_dev,
            transfer_rate: if has_all_response_sizes && duration_secs > 0.0 {
                Some(total_data as f64 / duration_secs)
            } else {
                None
            },
            response_time_distribution,
        }
    }
} 