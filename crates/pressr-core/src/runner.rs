use std::time::{Duration, Instant};
use reqwest::{Client, Method, header::HeaderMap};
use futures::{stream, StreamExt};
use tracing::{debug, info, instrument, warn};

use crate::data::RequestData;
use crate::result::{RequestResult, LoadTestResults};
use crate::error::{Error, Result};

/// Configuration for the load test runner
#[derive(Debug, Clone)]
pub struct Config {
    /// URL to send requests to
    pub url: String,
    
    /// HTTP method to use
    pub method: Method,
    
    /// HTTP headers to include
    pub headers: HeaderMap,
    
    /// Number of requests to send
    pub request_count: usize,
    
    /// Number of concurrent requests
    pub concurrency: usize,
    
    /// Request timeout in seconds
    pub timeout: u64,
}

/// Load test runner
#[derive(Debug)]
pub struct Runner {
    /// HTTP client to use for requests
    client: Client,
    
    /// Configuration for the load test
    config: Config,
    
    /// Optional request data
    data: Option<RequestData>,
}

impl Runner {
    /// Create a new Runner
    pub fn new(client: Client, config: Config, data: Option<RequestData>) -> Self {
        Self {
            client,
            config,
            data,
        }
    }
    
    /// Create a new client with the specified timeout
    pub fn create_client(timeout: u64) -> Result<Client> {
        debug!("Creating HTTP client with timeout: {}s", timeout);
        Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(Error::HttpClient)
    }
    
    /// Run the load test
    #[instrument(skip_all, fields(
        url = %self.config.url,
        method = %self.config.method,
        requests = self.config.request_count,
        concurrency = self.config.concurrency
    ))]
    pub async fn run(&self) -> Result<LoadTestResults> {
        info!("Starting load test: {} requests, {} concurrent", 
              self.config.request_count, self.config.concurrency);
              
        let start = Instant::now();
        
        // Create a stream of request indices
        let indices: Vec<usize> = (0..self.config.request_count).collect();
        
        // Convert the indices into a stream
        let results = stream::iter(indices)
            .map(|i| self.execute_request(i))
            .buffer_unordered(self.config.concurrency)
            .collect::<Vec<Result<RequestResult>>>()
            .await;
            
        let duration = start.elapsed();
        
        // Process results, filtering out errors
        let mut request_results = Vec::with_capacity(results.len());
        let mut errors = 0;
        
        for result in results {
            match result {
                Ok(result) => {
                    if !result.success {
                        errors += 1;
                    }
                    request_results.push(result);
                },
                Err(e) => {
                    errors += 1;
                    warn!("Error executing request: {}", e);
                    request_results.push(RequestResult {
                        status: None,
                        response_time: 0,
                        success: false,
                        error: Some(e.to_string()),
                        response_size: None,
                    });
                }
            }
        }
        
        info!("Load test completed: {} requests, {} errors, duration: {:.2}s",
              self.config.request_count, errors, duration.as_secs_f64());
              
        // Create the load test results
        Ok(LoadTestResults::new(request_results, duration))
    }
    
    /// Execute a single request
    #[instrument(skip_all, fields(index = index))]
    async fn execute_request(&self, index: usize) -> Result<RequestResult> {
        debug!("Executing request {}/{}", index + 1, self.config.request_count);
        
        let start = Instant::now();
        let mut builder = self.client
            .request(self.config.method.clone(), &self.config.url)
            .headers(self.config.headers.clone());
        
        // Add body if available and method is appropriate
        if let Some(data) = &self.data {
            if matches!(self.config.method, Method::POST | Method::PUT | Method::PATCH) {
                if let Some(body) = &data.body {
                    debug!("Adding JSON body to request");
                    builder = builder.json(body);
                }
            }
        }
        
        // Execute the request
        let result = match builder.send().await {
            Ok(response) => {
                let status = response.status();
                let status_code = status.as_u16();
                
                // Read the response body
                match response.text().await {
                    Ok(body) => {
                        let duration = start.elapsed();
                        let response_time = duration.as_millis();
                        
                        debug!("Request completed with status {} in {} ms",
                               status, response_time);
                        
                        let success = status.is_success();
                        let error = if !success {
                            Some(format!("HTTP Error: {} {}", status_code, status.canonical_reason().unwrap_or("Unknown")))
                        } else {
                            None
                        };
                        
                        RequestResult {
                            status: Some(status_code),
                            response_time,
                            success,
                            error,
                            response_size: Some(body.len()),
                        }
                    },
                    Err(e) => {
                        let duration = start.elapsed();
                        let response_time = duration.as_millis();
                        
                        warn!("Error reading response body: {}", e);
                        
                        RequestResult {
                            status: Some(status_code),
                            response_time,
                            success: false,
                            error: Some(format!("Error reading response body: {}", e)),
                            response_size: None,
                        }
                    }
                }
            },
            Err(e) => {
                let duration = start.elapsed();
                let response_time = duration.as_millis();
                
                warn!("Request failed: {}", e);
                
                RequestResult {
                    status: None,
                    response_time,
                    success: false,
                    error: Some(e.to_string()),
                    response_size: None,
                }
            }
        };
        
        Ok(result)
    }
} 