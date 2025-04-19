use clap::{Parser, ValueEnum};
use std::path::PathBuf;

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

/// Supported output formats
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Text,
    Json,
    // Future formats: Csv, Html
}

fn main() {
    let args = Args::parse();
    
    // Output parsed arguments for testing
    println!("Starting pressr with the following configuration:");
    println!("URL: {}", args.url);
    println!("Method: {:?}", args.method);
    println!("Requests: {}", args.requests);
    println!("Concurrency: {}", args.concurrency);
    
    if let Some(data_file) = args.data_file.as_ref() {
        println!("Data file: {}", data_file.display());
    }
    
    if !args.headers.is_empty() {
        println!("Headers:");
        for header in &args.headers {
            println!("  {}", header);
        }
    }
    
    println!("Timeout: {} seconds", args.timeout);
    println!("Output format: {:?}", args.output);
    
    // TODO: Implement the actual load testing functionality
}
