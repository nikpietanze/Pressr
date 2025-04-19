use clap::{Parser, ValueEnum};
use reqwest::{Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use std::{path::PathBuf, str::FromStr, time::Duration};

mod data;
mod runner;
mod report;

use data::RequestData;
use runner::Runner;
use report::{ReportFormat, generate_report};

/// pressr - A load testing tool for APIs and applications
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL to send requests to
    #[arg(short, long)]
    url: String,

    /// HTTP method to use
    #[arg(short, long, value_enum, default_value_t = HttpMethod::Get)]
    method: HttpMethod,

    /// Number of requests to send
    #[arg(short, long, default_value_t = 100)]
    requests: usize,

    /// Number of concurrent requests
    #[arg(short, long, default_value_t = 10)]
    concurrency: usize,

    /// Path to data file (JSON or YAML) containing request data
    #[arg(short, long)]
    data_file: Option<PathBuf>,

    /// HTTP headers in the format "key:value"
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,

    /// Request timeout in seconds
    #[arg(short, long, default_value_t = 30)]
    timeout: u64,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,
}

/// Supported HTTP methods
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
}

impl HttpMethod {
    /// Convert HttpMethod to reqwest::Method
    fn to_reqwest_method(&self) -> Method {
        match self {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Delete => Method::DELETE,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Head => Method::HEAD,
        }
    }
}

/// Supported output formats
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Text,
    Json,
    // Future formats: Csv, Html
}

impl OutputFormat {
    /// Convert OutputFormat to ReportFormat
    fn to_report_format(&self) -> ReportFormat {
        match self {
            OutputFormat::Text => ReportFormat::Text,
            OutputFormat::Json => ReportFormat::Json,
        }
    }
}

/// Parse headers from command line strings (format: "key:value")
fn parse_headers(header_strings: &[String]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    
    for header_str in header_strings {
        if let Some(colon_pos) = header_str.find(':') {
            let (key, value) = header_str.split_at(colon_pos);
            // Skip the colon
            let value = value.trim_start_matches(':').trim();
            
            // Convert key to HeaderName and value to HeaderValue
            if let (Ok(key), Ok(value)) = (
                HeaderName::from_str(key.trim()),
                HeaderValue::from_str(value)
            ) {
                headers.insert(key, value);
            } else {
                eprintln!("Warning: Invalid header: {}", header_str);
            }
        } else {
            eprintln!("Warning: Invalid header format: {}. Expected 'key:value'", header_str);
        }
    }
    
    headers
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Starting pressr with the following configuration:");
    println!("URL: {}", args.url);
    println!("Method: {:?}", args.method);
    println!("Requests: {}", args.requests);
    println!("Concurrency: {}", args.concurrency);
    
    // Load data file if specified
    let request_data = match &args.data_file {
        Some(path) => {
            println!("Data file: {}", path.display());
            match RequestData::from_json_file(path).await {
                Ok(data) => {
                    println!("Successfully loaded data file");
                    
                    // Print a summary of what was loaded
                    if data.body.is_some() {
                        println!("  Request body defined in data file");
                    }
                    
                    if !data.headers.is_empty() {
                        println!("  {} header(s) defined in data file", data.headers.len());
                    }
                    
                    if !data.params.is_empty() {
                        println!("  {} URL parameter(s) defined in data file", data.params.len());
                    }
                    
                    if !data.path_variables.is_empty() {
                        println!("  {} path variable(s) defined in data file", data.path_variables.len());
                    }
                    
                    if !data.variables.is_empty() {
                        println!("  {} variable set(s) defined for randomization", data.variables.len());
                    }
                    
                    Some(data)
                },
                Err(err) => {
                    eprintln!("Error loading data file: {}", err);
                    None
                }
            }
        },
        None => None,
    };
    
    if !args.headers.is_empty() {
        println!("Headers from command line:");
        for header in &args.headers {
            println!("  {}", header);
        }
    }
    
    println!("Timeout: {} seconds", args.timeout);
    println!("Output format: {:?}", args.output);
    
    // Create a client with the specified timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(args.timeout))
        .build()?;
    
    // Parse command-line headers
    let mut headers = parse_headers(&args.headers);
    
    // Add headers from data file if available
    if let Some(data) = &request_data {
        for (key, value) in &data.headers {
            if let (Ok(key), Ok(value)) = (
                HeaderName::from_str(key),
                HeaderValue::from_str(value)
            ) {
                headers.insert(key, value);
            }
        }
    }
    
    // Send a single request as a test first
    println!("\nSending a test request to {}", args.url);
    
    let mut test_request_builder = client
        .request(args.method.to_reqwest_method(), &args.url)
        .headers(headers.clone());
    
    // Add body from data file if available and method is appropriate
    if let Some(data) = &request_data {
        if matches!(args.method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch) {
            if let Some(body) = &data.body {
                test_request_builder = test_request_builder.json(body);
            }
        }
    }
    
    let start = std::time::Instant::now();
    
    match test_request_builder.send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            let body = response.text().await?;
            
            println!("Test request completed in {} ms", duration.as_millis());
            println!("Status: {} ({})", status.as_u16(), status.canonical_reason().unwrap_or("Unknown"));
            println!("Response size: {} bytes", body.len());
            
            if body.len() <= 1000 {
                println!("Response body:");
                println!("{}", body);
            } else {
                println!("Response body: (truncated, {} bytes total)", body.len());
                println!("{}", &body[..1000]);
                println!("... [truncated]");
            }
            
            // Now proceed with the actual load test
            println!("\nStarting load test with {} requests ({} concurrent)...", args.requests, args.concurrency);
            
            // Create and run the load test
            let runner = Runner::new(
                client,
                args.url,
                args.method.to_reqwest_method(),
                headers,
                request_data,
                args.requests,
                args.concurrency,
            );
            
            let test_start = std::time::Instant::now();
            let results = runner.run().await;
            let test_duration = test_start.elapsed();
            
            println!("\nLoad test completed in {:.2} seconds", test_duration.as_secs_f64());
            
            // Generate and output report
            let report = generate_report(&results, args.output.to_report_format());
            println!("\n{}", report);
        },
        Err(e) => {
            eprintln!("Test request failed: {}", e);
            eprintln!("Cannot proceed with load test due to test request failure");
        }
    }
    
    Ok(())
}
