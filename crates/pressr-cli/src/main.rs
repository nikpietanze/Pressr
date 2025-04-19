use clap::{Parser, ValueEnum};
use reqwest::{Method, header::{HeaderMap, HeaderName, HeaderValue}};
use std::{path::PathBuf, str::FromStr};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

// Import pressr-core
use pressr_core::{Result, Error, RequestData, Runner, Config, ReportFormat as CoreReportFormat, ReportOptions};

mod report;
mod error;

use report::{ReportFormat, generate_report};
use error::AppError;

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
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Html)]
    output: OutputFormat,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Output file for the report (if not specified, auto-generates filename in reports directory)
    #[arg(short = 'f', long)]
    output_file: Option<String>,
    
    /// Disable histograms in the report
    #[arg(long)]
    no_histograms: bool,
    
    /// Include detailed information about each request in the report
    #[arg(long)]
    detailed: bool,
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
    Html,
    Svg,
}

impl OutputFormat {
    /// Convert OutputFormat to ReportFormat
    fn to_report_format(&self) -> ReportFormat {
        match self {
            OutputFormat::Text => ReportFormat::Text,
            OutputFormat::Json => ReportFormat::Json,
            OutputFormat::Html => ReportFormat::Html,
            OutputFormat::Svg => ReportFormat::Svg,
        }
    }
    
    /// Convert OutputFormat to CoreReportFormat
    fn to_core_report_format(&self) -> CoreReportFormat {
        match self {
            OutputFormat::Text => CoreReportFormat::Text,
            OutputFormat::Json => CoreReportFormat::Json,
            OutputFormat::Html => CoreReportFormat::Html,
            OutputFormat::Svg => CoreReportFormat::Svg,
        }
    }
}

/// Parse headers from command line strings (format: "key:value")
fn parse_headers(header_strings: &[String]) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    
    for header_str in header_strings {
        if let Some(colon_pos) = header_str.find(':') {
            let (key, value) = header_str.split_at(colon_pos);
            // Skip the colon
            let value = value.trim_start_matches(':').trim();
            
            // Convert key to HeaderName and value to HeaderValue
            match (
                HeaderName::from_str(key.trim()),
                HeaderValue::from_str(value)
            ) {
                (Ok(key), Ok(value)) => {
                    debug!("Added header: {}: {}", key, value.to_str().unwrap_or("<binary>"));
                    headers.insert(key, value);
                },
                _ => {
                    warn!("Invalid header: {}", header_str);
                    eprintln!("Warning: Invalid header: {}", header_str);
                }
            }
        } else {
            warn!("Invalid header format: {}", header_str);
            eprintln!("Warning: Invalid header format: {}. Expected 'key:value'", header_str);
        }
    }
    
    Ok(headers)
}

/// Initialize the logger
fn init_logger(verbose: bool) {
    let filter = if verbose {
        EnvFilter::from_default_env()
            .add_directive("pressr_cli=debug".parse().unwrap())
            .add_directive("pressr_core=debug".parse().unwrap())
            .add_directive("warn".parse().unwrap())
    } else {
        EnvFilter::from_default_env()
            .add_directive("pressr_cli=info".parse().unwrap())
            .add_directive("pressr_core=info".parse().unwrap())
            .add_directive("warn".parse().unwrap())
    };
    
    fmt()
        .with_target(false) // Don't show targets
        .with_env_filter(filter)
        .init();
}

#[tokio::main]
async fn main() -> std::result::Result<(), AppError> {
    let args = Args::parse();
    
    // Initialize the logger based on verbosity
    init_logger(args.verbose);
    
    info!("Starting pressr with URL: {}, Method: {:?}", args.url, args.method);
    debug!("Configuration: {} requests, {} concurrent, timeout: {}s", 
           args.requests, args.concurrency, args.timeout);
    
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
                    error!("Failed to load data file: {}", err);
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
    
    if args.no_histograms {
        println!("Histograms: Disabled");
    }
    
    if args.detailed {
        println!("Detailed report: Enabled");
    }
    
    if let Some(file) = &args.output_file {
        println!("Output file: {}", file);
    }
    
    // Create a client with the specified timeout
    debug!("Creating HTTP client with timeout: {}s", args.timeout);
    let client = Runner::create_client(args.timeout)
        .map_err(|e| {
            error!("Failed to create HTTP client: {}", e);
            AppError::Core(e)
        })?;
    
    // Parse command-line headers
    debug!("Parsing command-line headers");
    let mut headers = parse_headers(&args.headers).map_err(AppError::Core)?;
    
    // Add headers from data file if available
    if let Some(data) = &request_data {
        debug!("Adding headers from data file");
        for (key, value) in &data.headers {
            match (
                HeaderName::from_str(key),
                HeaderValue::from_str(value)
            ) {
                (Ok(key), Ok(value)) => {
                    debug!("Added header from data file: {}: {}", key, value.to_str().unwrap_or("<binary>"));
                    headers.insert(key, value);
                },
                _ => {
                    warn!("Invalid header in data file: {}: {}", key, value);
                }
            }
        }
    }
    
    // Send a single request as a test first
    println!("\nSending a test request to {}", args.url);
    info!("Sending test request to {}", args.url);
    
    let mut test_request_builder = client
        .request(args.method.to_reqwest_method(), &args.url)
        .headers(headers.clone());
    
    // Add body from data file if available and method is appropriate
    if let Some(data) = &request_data {
        if matches!(args.method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch) {
            if let Some(body) = &data.body {
                debug!("Adding JSON body to test request");
                test_request_builder = test_request_builder.json(body);
            }
        }
    }
    
    let start = std::time::Instant::now();
    
    match test_request_builder.send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            info!("Test request completed with status {} in {} ms", 
                  status, duration.as_millis());
            
            let body = response.text().await
                .map_err(|e| {
                    error!("Failed to read test response body: {}", e);
                    AppError::Core(Error::HttpClient(e))
                })?;
            
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
            
            // Create the runner config
            let config = Config {
                url: args.url,
                method: args.method.to_reqwest_method(),
                headers,
                request_count: args.requests,
                concurrency: args.concurrency,
                timeout: args.timeout,
            };
            
            // Create and run the load test
            let runner = Runner::new(client, config, request_data);
            
            let test_start = std::time::Instant::now();
            let results = runner.run().await.map_err(AppError::Core)?;
            let test_duration = test_start.elapsed();
            
            println!("\nLoad test completed in {:.2} seconds", test_duration.as_secs_f64());
            info!("Load test completed in {:.2} seconds", test_duration.as_secs_f64());
            
            // Create the report options
            let report_options = ReportOptions {
                format: args.output.to_core_report_format(),
                output_file: args.output_file.clone(),
                include_histograms: !args.no_histograms,
                include_details: args.detailed,
            };
            
            // Generate the report
            info!("Generating report with format: {:?}", args.output);
            let report = pressr_core::generate_report(&results, &report_options)
                .map_err(AppError::Core)?;
            
            // Print the report to stdout if no output file was specified
            if args.output_file.is_none() {
                println!("\n{}", report);
            } else {
                let output_path = if args.output_file.as_ref().unwrap().contains('/') || args.output_file.as_ref().unwrap().contains('\\') {
                    args.output_file.as_ref().unwrap().clone()
                } else {
                    format!("reports/{}", args.output_file.as_ref().unwrap())
                };
                println!("\nReport written to {}", output_path);
            }
            
            // The report has been saved to a file (path is logged by the core library)
            println!("\nReport generated successfully.");
        },
        Err(e) => {
            error!("Test request failed: {}", e);
            eprintln!("Test request failed: {}", e);
            eprintln!("Cannot proceed with load test due to test request failure");
            return Err(AppError::Core(Error::HttpClient(e)));
        }
    }
    
    Ok(())
}
