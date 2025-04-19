use crate::result::{LoadTestResults, RequestResult};
use crate::error::{Error, Result};
use hdrhistogram::Histogram;
use plotters::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::{debug, info, instrument};
use maud::{html, Markup, DOCTYPE, PreEscaped};
use serde::Serialize;
use std::time::Duration;

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Plain text report
    Text,
    /// JSON report
    Json,
    /// HTML report
    Html,
    /// SVG (histograms only)
    Svg,
}

/// Report output options
#[derive(Debug, Clone)]
pub struct ReportOptions {
    /// Format of the report
    pub format: ReportFormat,
    
    /// Output file path (None for stdout)
    pub output_file: Option<String>,
    
    /// Whether to include histograms
    pub include_histograms: bool,
    
    /// Whether to include detailed per-request information
    pub include_details: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            format: ReportFormat::Text,
            output_file: None,
            include_histograms: false,
            include_details: false,
        }
    }
}

/// Summary statistics for test results
#[derive(Debug, Clone)]
pub struct TestSummary {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_time: Duration,
    pub mean_response_time: f64,
    pub min_response_time: f64,
    pub max_response_time: f64,
    pub rps: f64,
    pub percentiles: Option<HashMap<String, f64>>,
}

impl TestSummary {
    /// Create a new TestSummary from test results
    pub fn new(results: &[RequestResult], total_time: Duration) -> Self {
        let total_requests = results.len();
        let successful_requests = results.iter().filter(|r| r.success).count();
        let failed_requests = total_requests - successful_requests;
        
        let total_response_time: f64 = results.iter().map(|r| r.response_time as f64).sum();
        let mean_response_time = if total_requests > 0 {
            total_response_time / total_requests as f64
        } else {
            0.0
        };
        
        let min_response_time = results
            .iter()
            .map(|r| r.response_time as f64)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
            
        let max_response_time = results
            .iter()
            .map(|r| r.response_time as f64)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
            
        let rps = if total_time.as_secs_f64() > 0.0 {
            total_requests as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };
        
        let percentiles = if !results.is_empty() {
            // Create a histogram for percentile calculation
            match create_histogram_from_results(results) {
                Some(hist) => {
                    let mut map = HashMap::new();
                    
                    // Extract various percentiles and convert u64 to f64
                    map.insert("p50".to_string(), hist.value_at_percentile(50.0) as f64);
                    map.insert("p90".to_string(), hist.value_at_percentile(90.0) as f64);
                    map.insert("p95".to_string(), hist.value_at_percentile(95.0) as f64);
                    map.insert("p99".to_string(), hist.value_at_percentile(99.0) as f64);
                    
                    Some(map)
                },
                None => None
            }
        } else {
            None
        };
        
        TestSummary {
            total_requests,
            successful_requests,
            failed_requests,
            total_time,
            mean_response_time,
            min_response_time,
            max_response_time,
            rps,
            percentiles,
        }
    }
}

/// Generate a report based on the specified options
#[instrument(skip(results, options))]
pub fn generate_report(results: &LoadTestResults, options: &ReportOptions) -> Result<String> {
    info!("Generating {:?} report for load test with {} requests", 
          options.format, results.total_requests);
    
    let report = match options.format {
        ReportFormat::Text => generate_text_report(results, options),
        ReportFormat::Json => generate_json_report(results, options),
        ReportFormat::Html => generate_html_report(results, options),
        ReportFormat::Svg => {
            if !options.include_histograms {
                return Err(Error::Other("SVG format is only available for histograms".to_string()));
            }
            generate_histogram_svg(results)
        }
    }?;
    
    // Write to file if specified
    if let Some(path) = &options.output_file {
        debug!("Writing report to file: {}", path);
        let mut file = File::create(path)
            .map_err(|e| Error::Io(e))?;
        file.write_all(report.as_bytes())
            .map_err(|e| Error::Io(e))?;
        info!("Report written to {}", path);
    }
    
    Ok(report)
}

/// Generate a text report
#[instrument(skip(results, options))]
fn generate_text_report(results: &LoadTestResults, options: &ReportOptions) -> Result<String> {
    debug!("Generating text report");
    let mut report = String::new();
    
    // Header
    report.push_str(&format!("LOAD TEST REPORT\n"));
    report.push_str(&format!("Requests: {}\n", results.total_requests));
    report.push_str("\n");
    
    // Summary
    report.push_str("SUMMARY\n");
    report.push_str(&format!("Total requests:     {}\n", results.total_requests));
    report.push_str(&format!("Successful:        {} ({:.1}%)\n", 
        results.successful_requests, 
        percentage(results.successful_requests, results.total_requests)
    ));
    report.push_str(&format!("Failed:            {} ({:.1}%)\n", 
        results.failed_requests, 
        percentage(results.failed_requests, results.total_requests)
    ));
    report.push_str("\n");
    
    // Timing
    report.push_str("TIMING\n");
    report.push_str(&format!("Total duration:     {:.2} s\n", results.duration_secs));
    report.push_str(&format!("Average:            {:.2} ms\n", results.average_response_time));
    report.push_str(&format!("Minimum:            {} ms\n", results.min_response_time));
    report.push_str(&format!("Maximum:            {} ms\n", results.max_response_time));
    
    // Add percentiles
    if let Some(hist) = create_histogram(results) {
        report.push_str(&format!("50th percentile:     {:.2} ms\n", hist.value_at_percentile(50.0) as f64));
        report.push_str(&format!("90th percentile:     {:.2} ms\n", hist.value_at_percentile(90.0) as f64));
        report.push_str(&format!("95th percentile:     {:.2} ms\n", hist.value_at_percentile(95.0) as f64));
        report.push_str(&format!("99th percentile:     {:.2} ms\n", hist.value_at_percentile(99.0) as f64));
    }
    report.push_str("\n");
    
    // Status codes
    if !results.status_codes.is_empty() {
        report.push_str("STATUS CODES\n");
        
        // Sort status codes for consistent output
        let mut sorted_status_codes: Vec<_> = results.status_codes.iter().collect();
        sorted_status_codes.sort_by_key(|&(code, _)| *code);
        
        for (code, count) in sorted_status_codes {
            let percent = percentage(*count, results.total_requests);
            report.push_str(&format!("{}: {} ({:.1}%)\n", code, count, percent));
        }
        report.push_str("\n");
    }
    
    // Error summary
    if !results.errors.is_empty() {
        report.push_str("ERRORS\n");
        
        for (error, count) in &results.errors {
            let percent = percentage(*count, results.total_requests);
            report.push_str(&format!("{}: {} ({:.1}%)\n", error, count, percent));
        }
        report.push_str("\n");
    }
    
    // Add detailed per-request information if requested
    if options.include_details {
        report.push_str("REQUEST DETAILS\n");
        for (i, result) in results.requests.iter().enumerate() {
            report.push_str(&format!("Request #{}: ", i + 1));
            if result.success {
                report.push_str(&format!("Success, Status: {}, ", 
                                        result.status.map(|s| s.to_string()).unwrap_or_else(|| "None".to_string())));
            } else {
                report.push_str(&format!("Failed, Error: {}, ", 
                                        result.error.as_ref().map(|e| e.as_str()).unwrap_or("None")));
            }
            report.push_str(&format!("Time: {} ms", result.response_time));
            if let Some(size) = result.response_size {
                report.push_str(&format!(", Size: {} bytes", size));
            }
            report.push_str("\n");
        }
        report.push_str("\n");
    }
    
    debug!("Text report generated ({} chars)", report.len());
    Ok(report)
}

/// Generate a JSON report
#[instrument(skip(results, options))]
fn generate_json_report(results: &LoadTestResults, options: &ReportOptions) -> Result<String> {
    debug!("Generating JSON report");
    
    #[derive(Serialize)]
    struct JsonReport<'a> {
        completed_requests: usize,
        successful_requests: usize,
        failed_requests: usize,
        total_duration_secs: f64,
        avg_duration_ms: f64,
        min_duration_ms: u128,
        max_duration_ms: u128,
        percentiles: Option<HashMap<String, f64>>,
        success_rate: f64,
        failure_rate: f64,
        status_codes: HashMap<u16, usize>,
        error_counts: HashMap<String, usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_details: Option<&'a Vec<RequestResult>>,
    }
    
    // Calculate percentiles if histograms are enabled
    let percentiles = if options.include_histograms {
        if let Some(hist) = create_histogram(results) {
            let mut map = HashMap::new();
            
            map.insert("p50".to_string(), hist.value_at_percentile(50.0) as f64);
            map.insert("p90".to_string(), hist.value_at_percentile(90.0) as f64);
            map.insert("p95".to_string(), hist.value_at_percentile(95.0) as f64);
            map.insert("p99".to_string(), hist.value_at_percentile(99.0) as f64);
            
            Some(map)
        } else {
            None
        }
    } else {
        None
    };
    
    // Convert status_codes HashMap to BTreeMap for deterministic ordering
    let status_codes = results.status_codes.clone();
    
    // Convert errors HashMap to BTreeMap for deterministic ordering
    let error_counts = results.errors.clone();
    
    // Calculate percentages
    let success_rate = percentage(results.successful_requests, results.total_requests);
    let failure_rate = percentage(results.failed_requests, results.total_requests);
    
    // Optional detailed results
    let request_details = if options.include_details {
        Some(&results.requests)
    } else {
        None
    };
    
    // Create the JSON report
    let report = JsonReport {
        completed_requests: results.total_requests,
        successful_requests: results.successful_requests,
        failed_requests: results.failed_requests,
        total_duration_secs: results.duration_secs,
        avg_duration_ms: results.average_response_time,
        min_duration_ms: results.min_response_time,
        max_duration_ms: results.max_response_time,
        percentiles,
        success_rate,
        failure_rate,
        status_codes,
        error_counts,
        request_details,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| Error::Json(e))?;
    
    debug!("JSON report generated ({} chars)", json.len());
    Ok(json)
}

/// Generate an HTML report
#[instrument(skip(results, options))]
fn generate_html_report(results: &LoadTestResults, options: &ReportOptions) -> Result<String> {
    debug!("Generating HTML report");
    
    // Calculate percentiles
    let percentiles = if options.include_histograms {
        if let Some(hist) = create_histogram(results) {
            let mut map = HashMap::new();
            
            map.insert("p50".to_string(), hist.value_at_percentile(50.0) as f64);
            map.insert("p90".to_string(), hist.value_at_percentile(90.0) as f64);
            map.insert("p95".to_string(), hist.value_at_percentile(95.0) as f64);
            map.insert("p99".to_string(), hist.value_at_percentile(99.0) as f64);
            
            Some(map)
        } else {
            None
        }
    } else {
        None
    };
    
    // Generate SVG for histogram if enabled
    let histogram_svg = if options.include_histograms {
        match generate_histogram_svg(results) {
            Ok(svg) => Some(svg),
            Err(e) => {
                debug!("Failed to generate histogram SVG: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Build HTML manually instead of using Maud
    let mut html = String::new();
    
    // HTML preamble
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n<head>\n");
    html.push_str("  <meta charset=\"utf-8\">\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("  <title>Load Test Report</title>\n");
    html.push_str("  <style>\n");
    html.push_str("    body {\n");
    html.push_str("      font-family: -apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, Helvetica, Arial, sans-serif;\n");
    html.push_str("      line-height: 1.5;\n");
    html.push_str("      color: #333;\n");
    html.push_str("      max-width: 1200px;\n");
    html.push_str("      margin: 0 auto;\n");
    html.push_str("      padding: 1em;\n");
    html.push_str("    }\n");
    html.push_str("    h1, h2, h3 { color: #2c3e50; }\n");
    html.push_str("    h1 { border-bottom: 2px solid #eaecef; padding-bottom: 0.3em; }\n");
    html.push_str("    h2 { border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; }\n");
    html.push_str("    .summary-grid {\n");
    html.push_str("      display: grid;\n");
    html.push_str("      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));\n");
    html.push_str("      gap: 1rem;\n");
    html.push_str("      margin: 1rem 0;\n");
    html.push_str("    }\n");
    html.push_str("    .summary-item {\n");
    html.push_str("      background-color: #f8f9fa;\n");
    html.push_str("      border-radius: 4px;\n");
    html.push_str("      padding: 1rem;\n");
    html.push_str("      box-shadow: 0 1px 3px rgba(0,0,0,0.12);\n");
    html.push_str("    }\n");
    html.push_str("    .summary-item h3 {\n");
    html.push_str("      margin-top: 0;\n");
    html.push_str("      color: #6c757d;\n");
    html.push_str("      font-size: 0.9rem;\n");
    html.push_str("      text-transform: uppercase;\n");
    html.push_str("      letter-spacing: 0.05em;\n");
    html.push_str("    }\n");
    html.push_str("    .summary-item p {\n");
    html.push_str("      margin-bottom: 0;\n");
    html.push_str("      font-size: 1.25rem;\n");
    html.push_str("      font-weight: 500;\n");
    html.push_str("    }\n");
    html.push_str("    table {\n");
    html.push_str("      width: 100%;\n");
    html.push_str("      border-collapse: collapse;\n");
    html.push_str("      margin: 1rem 0;\n");
    html.push_str("    }\n");
    html.push_str("    th, td {\n");
    html.push_str("      text-align: left;\n");
    html.push_str("      padding: 0.5rem;\n");
    html.push_str("      border-bottom: 1px solid #ddd;\n");
    html.push_str("    }\n");
    html.push_str("    th { background-color: #f8f9fa; font-weight: 500; }\n");
    html.push_str("    tr:nth-child(even) { background-color: #f8f9fa; }\n");
    html.push_str("    .success { color: #28a745; }\n");
    html.push_str("    .failure { color: #dc3545; }\n");
    html.push_str("    .histogram-container {\n");
    html.push_str("      margin: 2rem 0;\n");
    html.push_str("      max-width: 100%;\n");
    html.push_str("      overflow-x: auto;\n");
    html.push_str("    }\n");
    html.push_str("    svg { max-width: 100%; height: auto; }\n");
    html.push_str("  </style>\n");
    html.push_str("</head>\n<body>\n");
    
    // Report header
    html.push_str("  <h1>Load Test Report</h1>\n");
    
    // Summary section
    html.push_str("  <h2>Summary</h2>\n");
    html.push_str("  <div class=\"summary-grid\">\n");
    
    // Total requests
    html.push_str("    <div class=\"summary-item\">\n");
    html.push_str("      <h3>Total Requests</h3>\n");
    html.push_str(&format!("      <p>{}</p>\n", results.total_requests));
    html.push_str("    </div>\n");
    
    // Success rate
    html.push_str("    <div class=\"summary-item\">\n");
    html.push_str("      <h3>Success Rate</h3>\n");
    html.push_str(&format!("      <p><span class=\"success\">{:.1}%</span></p>\n", 
        percentage(results.successful_requests, results.total_requests)));
    html.push_str("    </div>\n");
    
    // Duration
    html.push_str("    <div class=\"summary-item\">\n");
    html.push_str("      <h3>Duration</h3>\n");
    html.push_str(&format!("      <p>{:.2}s</p>\n", results.duration_secs));
    html.push_str("    </div>\n");
    
    // Average response time
    html.push_str("    <div class=\"summary-item\">\n");
    html.push_str("      <h3>Avg Response</h3>\n");
    html.push_str(&format!("      <p>{:.2} ms</p>\n", results.average_response_time));
    html.push_str("    </div>\n");
    
    // Min response time
    html.push_str("    <div class=\"summary-item\">\n");
    html.push_str("      <h3>Min Response</h3>\n");
    html.push_str(&format!("      <p>{} ms</p>\n", results.min_response_time));
    html.push_str("    </div>\n");
    
    // Max response time
    html.push_str("    <div class=\"summary-item\">\n");
    html.push_str("      <h3>Max Response</h3>\n");
    html.push_str(&format!("      <p>{} ms</p>\n", results.max_response_time));
    html.push_str("    </div>\n");
    
    // Percentiles
    if let Some(p) = &percentiles {
        // 50th percentile
        if let Some(p50) = p.get("p50") {
            html.push_str("    <div class=\"summary-item\">\n");
            html.push_str("      <h3>50th Pct</h3>\n");
            html.push_str(&format!("      <p>{:.2} ms</p>\n", p50));
            html.push_str("    </div>\n");
        }
        
        // 90th percentile
        if let Some(p90) = p.get("p90") {
            html.push_str("    <div class=\"summary-item\">\n");
            html.push_str("      <h3>90th Pct</h3>\n");
            html.push_str(&format!("      <p>{:.2} ms</p>\n", p90));
            html.push_str("    </div>\n");
        }
        
        // 95th percentile
        if let Some(p95) = p.get("p95") {
            html.push_str("    <div class=\"summary-item\">\n");
            html.push_str("      <h3>95th Pct</h3>\n");
            html.push_str(&format!("      <p>{:.2} ms</p>\n", p95));
            html.push_str("    </div>\n");
        }
        
        // 99th percentile
        if let Some(p99) = p.get("p99") {
            html.push_str("    <div class=\"summary-item\">\n");
            html.push_str("      <h3>99th Pct</h3>\n");
            html.push_str(&format!("      <p>{:.2} ms</p>\n", p99));
            html.push_str("    </div>\n");
        }
    }
    
    html.push_str("  </div>\n"); // Close summary-grid
    
    // Status Codes section
    if !results.status_codes.is_empty() {
        html.push_str("  <h2>Status Codes</h2>\n");
        html.push_str("  <table>\n");
        html.push_str("    <thead>\n");
        html.push_str("      <tr>\n");
        html.push_str("        <th>Status Code</th>\n");
        html.push_str("        <th>Count</th>\n");
        html.push_str("        <th>Percentage</th>\n");
        html.push_str("      </tr>\n");
        html.push_str("    </thead>\n");
        html.push_str("    <tbody>\n");
        
        for (code, count) in &results.status_codes {
            html.push_str("      <tr>\n");
            html.push_str(&format!("        <td>{}</td>\n", code));
            html.push_str(&format!("        <td>{}</td>\n", count));
            html.push_str(&format!("        <td>{:.1}%</td>\n", 
                percentage(*count, results.total_requests)));
            html.push_str("      </tr>\n");
        }
        
        html.push_str("    </tbody>\n");
        html.push_str("  </table>\n");
    }
    
    // Errors section
    if !results.errors.is_empty() {
        html.push_str("  <h2>Errors</h2>\n");
        html.push_str("  <table>\n");
        html.push_str("    <thead>\n");
        html.push_str("      <tr>\n");
        html.push_str("        <th>Error</th>\n");
        html.push_str("        <th>Count</th>\n");
        html.push_str("        <th>Percentage</th>\n");
        html.push_str("      </tr>\n");
        html.push_str("    </thead>\n");
        html.push_str("    <tbody>\n");
        
        for (error, count) in &results.errors {
            html.push_str("      <tr>\n");
            html.push_str(&format!("        <td>{}</td>\n", error));
            html.push_str(&format!("        <td>{}</td>\n", count));
            html.push_str(&format!("        <td>{:.1}%</td>\n", 
                percentage(*count, results.total_requests)));
            html.push_str("      </tr>\n");
        }
        
        html.push_str("    </tbody>\n");
        html.push_str("  </table>\n");
    }
    
    // Histogram section
    if let Some(svg) = &histogram_svg {
        html.push_str("  <h2>Response Time Distribution</h2>\n");
        html.push_str("  <div class=\"histogram-container\">\n");
        html.push_str(&svg);
        html.push_str("  </div>\n");
    }
    
    // Request details section
    if options.include_details {
        html.push_str("  <h2>Request Details</h2>\n");
        html.push_str("  <table>\n");
        html.push_str("    <thead>\n");
        html.push_str("      <tr>\n");
        html.push_str("        <th>#</th>\n");
        html.push_str("        <th>Status</th>\n");
        html.push_str("        <th>Response Time</th>\n");
        html.push_str("        <th>Size</th>\n");
        html.push_str("        <th>Result</th>\n");
        html.push_str("      </tr>\n");
        html.push_str("    </thead>\n");
        html.push_str("    <tbody>\n");
        
        for (i, result) in results.requests.iter().enumerate() {
            html.push_str("      <tr>\n");
            html.push_str(&format!("        <td>{}</td>\n", i + 1));
            
            // Status
            html.push_str("        <td>");
            if let Some(status) = result.status {
                html.push_str(&status.to_string());
            } else {
                html.push_str("N/A");
            }
            html.push_str("</td>\n");
            
            // Response time
            html.push_str(&format!("        <td>{} ms</td>\n", result.response_time));
            
            // Size
            html.push_str("        <td>");
            if let Some(size) = result.response_size {
                html.push_str(&format!("{} bytes", size));
            } else {
                html.push_str("N/A");
            }
            html.push_str("</td>\n");
            
            // Result
            html.push_str("        <td>");
            if result.success {
                html.push_str("<span class=\"success\">Success</span>");
            } else {
                html.push_str("<span class=\"failure\">Failed");
                if let Some(error) = &result.error {
                    html.push_str(&format!(": {}", error));
                }
                html.push_str("</span>");
            }
            html.push_str("</td>\n");
            
            html.push_str("      </tr>\n");
        }
        
        html.push_str("    </tbody>\n");
        html.push_str("  </table>\n");
    }
    
    // Footer
    html.push_str("  <footer>\n");
    html.push_str("    <hr>\n");
    html.push_str("    <p>Generated by <a href=\"https://github.com/username/pressr\">pressr</a></p>\n");
    html.push_str("  </footer>\n");
    
    // Close body and html tags
    html.push_str("</body>\n</html>");
    
    debug!("HTML report generated ({} chars)", html.len());
    Ok(html)
}

/// Generate a histogram SVG
#[instrument(skip(results))]
fn generate_histogram_svg(results: &LoadTestResults) -> Result<String> {
    debug!("Generating SVG histogram");
    
    // Create a buffer for the SVG
    let mut buffer = String::new();
    
    // Extract response times
    let response_times: Vec<f64> = results.requests.iter()
        .map(|r| r.response_time as f64)
        .collect();
    
    if response_times.is_empty() {
        return Err(Error::Other("No data available for histogram".to_string()));
    }
    
    // Find min and max times
    let min_time = *response_times.iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let max_time = *response_times.iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    
    let range = max_time - min_time;
    if range <= 0.0 {
        return Err(Error::Other("Cannot generate histogram with zero range".to_string()));
    }
    
    // Number of buckets for the histogram
    let bucket_count = 20;
    
    // Create SVG
    {
        let root = SVGBackend::with_string(&mut buffer, (800, 400))
            .into_drawing_area();
        
        root.fill(&WHITE)
            .map_err(|_| Error::Other("Failed to fill SVG background".to_string()))?;
        
        // Add a bit of padding to max
        let padding = range * 0.05;
        let max_time_with_padding = max_time + padding;
        
        // Create histogram data by bucket
        let bucket_size = (max_time_with_padding - min_time) / bucket_count as f64;
        let mut histogram_data = Vec::new();
        let mut x = min_time;
        
        for _ in 0..bucket_count {
            let next_x = x + bucket_size;
            let mut count = 0;
            
            // Count samples in this bucket
            for &time in &response_times {
                if time >= x && time < next_x {
                    count += 1;
                }
            }
            
            histogram_data.push((x, count));
            x = next_x;
        }
        
        // Determine y scale
        let max_count = histogram_data.iter()
            .map(|(_, count)| *count)
            .max()
            .unwrap_or(0);
        
        // Create the chart
        let mut chart = ChartBuilder::on(&root)
            .caption("Response Time Distribution", ("sans-serif", 22))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                min_time..max_time_with_padding,
                0.0..max_count as f64 * 1.1
            )
            .map_err(|_| Error::Other("Failed to create chart".to_string()))?;
        
        chart.configure_mesh()
            .x_desc("Response Time (ms)")
            .y_desc("Request Count")
            .draw()
            .map_err(|_| Error::Other("Failed to draw chart mesh".to_string()))?;
        
        // Draw histogram bars
        chart.draw_series(
            histogram_data.iter().map(|&(x, count)| {
                Rectangle::new(
                    [(x, 0.0), (x + bucket_size, count as f64)],
                    BLUE.mix(0.3).filled(),
                )
            })
        )
        .map_err(|_| Error::Other("Failed to draw histogram bars".to_string()))?
        .label("Response Times");
        
        // Draw the percentile lines
        if let Some(hist) = create_histogram(results) {
            // Define orange color (missing in plotters)
            let orange = RGBColor(255, 165, 0);
            
            let p50 = hist.value_at_percentile(50.0) as f64;
            let p90 = hist.value_at_percentile(90.0) as f64;
            let p95 = hist.value_at_percentile(95.0) as f64;
            let p99 = hist.value_at_percentile(99.0) as f64;
            
            // Draw 50th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p50, 0.0), (p50, max_count as f64)],
                GREEN.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p50 line".to_string()))?
            .label("50th Percentile");
            
            // Draw 90th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p90, 0.0), (p90, max_count as f64)],
                YELLOW.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p90 line".to_string()))?
            .label("90th Percentile");
            
            // Draw 95th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p95, 0.0), (p95, max_count as f64)],
                orange.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p95 line".to_string()))?
            .label("95th Percentile");
            
            // Draw 99th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p99, 0.0), (p99, max_count as f64)],
                RED.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p99 line".to_string()))?
            .label("99th Percentile");
        }
        
        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(WHITE.filled())
            .border_style(BLACK)
            .draw()
            .map_err(|_| Error::Other("Failed to draw chart legend".to_string()))?;
        
        // Ensure drawing is complete
        root.present()
            .map_err(|_| Error::Other("Failed to finalize SVG".to_string()))?;
    }
    
    // Return the SVG as a string
    debug!("SVG histogram generated ({} chars)", buffer.len());
    Ok(buffer)
}

/// Create a histogram from the response times
fn create_histogram(results: &LoadTestResults) -> Option<Histogram<u64>> {
    if results.requests.is_empty() {
        return None;
    }
    
    // Create histogram with appropriate precision
    // 3 significant figures of precision and value range from 1ms to 1 hour
    let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000, 3).ok()?;
    
    for result in &results.requests {
        // Record each response time in milliseconds (convert from u128 to u64)
        if let Err(e) = hist.record(result.response_time as u64) {
            debug!("Failed to record in histogram: {}", e);
        }
    }
    
    Some(hist)
}

/// Create a histogram from raw results array
fn create_histogram_from_results(results: &[RequestResult]) -> Option<Histogram<u64>> {
    if results.is_empty() {
        return None;
    }
    
    // Create histogram with appropriate precision
    // 3 significant figures of precision and value range from 1ms to 1 hour
    let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000, 3).ok()?;
    
    for result in results {
        // Record each response time in milliseconds (convert from u128 to u64)
        if let Err(e) = hist.record(result.response_time as u64) {
            debug!("Failed to record in histogram: {}", e);
        }
    }
    
    Some(hist)
}

/// Calculate percentage
fn percentage(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
} 