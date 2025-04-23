// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use pressr_core::{
    Runner, Config, Error as PressrError, LoadTestResults
};
use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GuiError {
    #[error("Core error: {0}")]
    Core(#[from] PressrError),
    
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
}

impl Serialize for GuiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct LoadTestParams {
    url: String,
    method: String,
    requests: u64,
    concurrency: u64,
    timeout_ms: Option<u64>,
    headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct LoadTestResponse {
    results: TestResults,
}

#[derive(Debug, Serialize)]
struct TestResults {
    request_count: u64,
    success_count: u64,
    failure_count: u64,
    total_time: f64,
    average_time: f64,
    min_time: f64,
    max_time: f64,
    throughput: f64,
    success_rate: f64,
    status_counts: BTreeMap<String, u64>,
    error_counts: BTreeMap<String, u64>,
}

#[tauri::command]
async fn run_load_test(params: LoadTestParams) -> Result<LoadTestResponse, GuiError> {
    println!("Received request to test URL: {}", params.url);
    
    // Parse HTTP method
    let method = Method::from_str(&params.method.to_uppercase())
        .map_err(|_| GuiError::InvalidMethod(params.method.clone()))?;
    
    // Configure timeout
    let timeout = params.timeout_ms.unwrap_or(30000);
    
    // Convert headers if provided
    let mut headers = HeaderMap::new();
    if let Some(header_map) = &params.headers {
        for (key, value) in header_map {
            let header_name = HeaderName::from_str(key)
                .map_err(|_| GuiError::InvalidHeader(format!("Invalid header name: {}", key)))?;
            
            let header_value = HeaderValue::from_str(value)
                .map_err(|_| GuiError::InvalidHeader(format!("Invalid header value for {}: {}", key, value)))?;
            
            headers.insert(header_name, header_value);
        }
    }
    
    // Create the client
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|e| GuiError::Core(PressrError::HttpClient(e)))?;
    
    // Create the config
    let config = Config {
        url: params.url,
        method,
        headers,
        request_count: params.requests as usize,
        concurrency: params.concurrency as usize,
        timeout: timeout / 1000, // Convert to seconds for the Config
    };
    
    // Create the runner
    let runner = Runner::new(client, config, None);
    
    // Run the load test
    let result = runner.run().await.map_err(GuiError::Core)?;
    
    // Convert the result to our response format
    let response = convert_result_to_response(result);
    
    Ok(response)
}

// Helper function to convert core result to GUI response
fn convert_result_to_response(result: LoadTestResults) -> LoadTestResponse {
    // Convert status counts map
    let status_counts = result.status_codes
        .into_iter()
        .map(|(status, count)| (status.to_string(), count as u64))
        .collect::<BTreeMap<String, u64>>();
    
    // Convert error counts map
    let error_counts = result.errors
        .into_iter()
        .map(|(error, count)| (error, count as u64))
        .collect::<BTreeMap<String, u64>>();
    
    LoadTestResponse {
        results: TestResults {
            request_count: result.total_requests as u64,
            success_count: result.successful_requests as u64,
            failure_count: result.failed_requests as u64,
            total_time: result.duration_secs * 1000.0, // Convert to ms
            average_time: result.average_response_time,
            min_time: result.min_response_time as f64,
            max_time: result.max_response_time as f64,
            throughput: result.throughput,
            success_rate: if result.total_requests > 0 {
                result.successful_requests as f64 / result.total_requests as f64
            } else {
                0.0
            },
            status_counts,
            error_counts,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![run_load_test])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
