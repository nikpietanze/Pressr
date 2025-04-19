use crate::data::RequestData;
use crate::error::{Result, RunnerError};
use futures::{stream, StreamExt};
use reqwest::{Client, Method, header::HeaderMap};
use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, trace, warn};

/// Result of a single HTTP request
#[derive(Debug, Clone)]
pub struct RequestResult {
    /// HTTP status code
    pub status: u16,
    
    /// Status text (e.g., "OK", "Not Found")
    pub status_text: String,
    
    /// Time taken to complete the request in milliseconds
    pub duration_ms: u128,
    
    /// Size of the response body in bytes
    pub response_size: usize,
    
    /// Error message if the request failed
    pub error: Option<String>,
}

impl RequestResult {
    /// Create a new successful request result
    pub fn success(status: u16, status_text: String, duration: Duration, response_size: usize) -> Self {
        debug!("Request succeeded with status {}", status);
        Self {
            status,
            status_text,
            duration_ms: duration.as_millis(),
            response_size,
            error: None,
        }
    }
    
    /// Create a new error request result
    pub fn error(error_message: String, duration: Duration) -> Self {
        error!("Request failed: {}", error_message);
        Self {
            status: 0,
            status_text: String::new(),
            duration_ms: duration.as_millis(),
            response_size: 0,
            error: Some(error_message),
        }
    }
}

/// Overall results from a load test
#[derive(Debug)]
pub struct LoadTestResults {
    /// URL that was tested
    pub url: String,
    
    /// HTTP method used
    pub method: String,
    
    /// Number of requests completed
    pub completed_requests: usize,
    
    /// Number of successful requests (2xx status codes)
    pub successful_requests: usize,
    
    /// Number of failed requests
    pub failed_requests: usize,
    
    /// Total time taken for all requests
    pub total_duration_ms: u128,
    
    /// Time taken by the fastest request
    pub min_duration_ms: u128,
    
    /// Time taken by the slowest request
    pub max_duration_ms: u128,
    
    /// Average request time
    pub avg_duration_ms: u128,
    
    /// Individual results for each request
    pub results: Vec<RequestResult>,
    
    /// Counts of each status code encountered
    pub status_code_counts: HashMap<u16, usize>,
}

impl LoadTestResults {
    /// Create a new empty results container
    pub fn new(url: String, method: String) -> Self {
        debug!("Creating new LoadTestResults for {} {}", method, url);
        Self {
            url,
            method,
            completed_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_duration_ms: 0,
            min_duration_ms: u128::MAX,
            max_duration_ms: 0,
            avg_duration_ms: 0,
            results: Vec::new(),
            status_code_counts: HashMap::new(),
        }
    }
    
    /// Add a request result and update the stats
    pub fn add_result(&mut self, result: RequestResult) {
        trace!("Adding result with status {} to results", result.status);
        
        // Update counters
        self.completed_requests += 1;
        
        if let Some(_) = &result.error {
            self.failed_requests += 1;
        } else if (200..300).contains(&result.status) {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        
        // Update durations
        self.total_duration_ms += result.duration_ms;
        self.min_duration_ms = self.min_duration_ms.min(result.duration_ms);
        self.max_duration_ms = self.max_duration_ms.max(result.duration_ms);
        
        // Update status code counts
        if result.status > 0 {
            *self.status_code_counts.entry(result.status).or_insert(0) += 1;
        }
        
        // Add the result to the list
        self.results.push(result);
        
        // Update average
        if self.completed_requests > 0 {
            self.avg_duration_ms = self.total_duration_ms / self.completed_requests as u128;
        }
    }
}

/// Load test runner 
pub struct Runner {
    client: Client,
    url: String,
    method: Method,
    headers: HeaderMap,
    request_data: Option<RequestData>,
    request_count: usize,
    concurrency: usize,
}

impl Runner {
    /// Create a new load test runner
    pub fn new(
        client: Client,
        url: String,
        method: Method,
        headers: HeaderMap,
        request_data: Option<RequestData>,
        request_count: usize,
        concurrency: usize,
    ) -> Self {
        info!("Creating runner for {} {} with {} requests ({} concurrent)",
            method, url, request_count, concurrency);
        Self {
            client,
            url,
            method,
            headers,
            request_data,
            request_count,
            concurrency,
        }
    }
    
    /// Run the load test with the specified parameters
    #[instrument(skip(self), fields(url = %self.url, method = ?self.method))]
    pub async fn run(&self) -> Result<LoadTestResults> {
        info!("Starting load test with {} requests ({} concurrent)", 
            self.request_count, self.concurrency);
        
        // Create shared results object
        let results = Arc::new(Mutex::new(
            LoadTestResults::new(
                self.url.clone(),
                format!("{:?}", self.method)
            )
        ));
        
        // Create a stream of indices for the requests
        let indices = 0..self.request_count;
        
        debug!("Setting up concurrent request stream");
        // Use buffered_unordered to limit concurrency while processing requests in order of completion
        stream::iter(indices)
            .map(|i| {
                let client = self.client.clone();
                let url = self.url.clone();
                let method = self.method.clone();
                let headers = self.headers.clone();
                let request_data = self.request_data.clone();
                let results = Arc::clone(&results);
                
                async move {
                    trace!("Executing request {}", i);
                    match self.execute_request(client, url, method, headers, request_data, i).await {
                        Ok(result) => {
                            // Add the result to the shared results
                            let mut results_lock = results.lock().await;
                            results_lock.add_result(result);
                            
                            // Print progress
                            if (results_lock.completed_requests % 10 == 0) || (results_lock.completed_requests == 1) {
                                info!("Completed {}/{} requests...", results_lock.completed_requests, self.request_count);
                            }
                        },
                        Err(e) => {
                            error!("Request {} failed: {}", i, e);
                            // Still need to update the counter even for failed requests
                            let mut results_lock = results.lock().await;
                            results_lock.add_result(RequestResult::error(
                                format!("Request failed: {}", e),
                                Duration::from_millis(0),
                            ));
                        }
                    }
                }
            })
            .buffer_unordered(self.concurrency)
            .collect::<Vec<()>>()
            .await;
        
        // Extract the results from the Arc<Mutex<>>
        let final_results = Arc::try_unwrap(results)
            .map_err(|_| RunnerError::ConcurrencyError("Failed to unwrap results".to_string()))?
            .into_inner();
        
        info!("Load test completed: {} requests, {} successful, {} failed",
            final_results.completed_requests,
            final_results.successful_requests,
            final_results.failed_requests);
        
        Ok(final_results)
    }
    
    /// Execute a single request and return the result
    #[instrument(skip(self, client, headers, request_data), fields(url = %url, method = ?method))]
    async fn execute_request(
        &self,
        client: Client,
        url: String,
        method: Method,
        headers: HeaderMap,
        request_data: Option<RequestData>,
        _request_index: usize, // Unused but kept for potential future use
    ) -> Result<RequestResult> {
        let start = Instant::now();
        
        // Build the request
        debug!("Building request");
        let mut request_builder = client
            .request(method, &url)
            .headers(headers);
        
        // Add body if available and applicable
        if matches!(self.method, Method::POST | Method::PUT | Method::PATCH) {
            if let Some(data) = &request_data {
                if let Some(body) = &data.body {
                    debug!("Adding JSON body to request");
                    request_builder = request_builder.json(body);
                }
            }
        }
        
        // Execute the request
        debug!("Sending request");
        match request_builder.send().await {
            Ok(response) => {
                debug!("Received response with status {}", response.status());
                let status = response.status();
                let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();
                
                // Get the body to calculate its size
                debug!("Reading response body");
                match response.bytes().await {
                    Ok(bytes) => {
                        let duration = start.elapsed();
                        let size = bytes.len();
                        
                        trace!("Response body size: {} bytes", size);
                        Ok(RequestResult::success(status.as_u16(), status_text, duration, size))
                    },
                    Err(e) => {
                        let duration = start.elapsed();
                        let error_msg = format!("Failed to read response body: {}", e);
                        error!("{}", error_msg);
                        Err(RunnerError::ResponseBodyFailed(error_msg).into())
                    }
                }
            },
            Err(e) => {
                let duration = start.elapsed();
                let error_msg = format!("Request failed: {}", e);
                error!("{}", error_msg);
                Err(RunnerError::RequestFailed(e).into())
            }
        }
    }
} 