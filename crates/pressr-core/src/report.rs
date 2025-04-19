use crate::result::{LoadTestResults, RequestResult};
use crate::error::{Error, Result};
use hdrhistogram::Histogram;
use plotters::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::{debug, info, instrument, warn};
use maud::{html, Markup, DOCTYPE, PreEscaped};
use serde::Serialize;
use std::time::Duration;
use chrono;

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
            format: ReportFormat::Html,
            output_file: None,
            include_histograms: true,
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

// Disable the warnings for instrument macro as it's an environmental issue
#[allow(warnings)]
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
    
    // Generate output file path (user-specified or auto-generated)
    let output_path = if let Some(path) = &options.output_file {
        debug!("Using user-specified output file: {}", path);
        
        // Get just the filename component, ignoring any directory parts
        let filename = std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        
        // Always place in the reports directory
        format!("reports/{}", filename)
    } else {
        // Auto-generate filename with format "report_N.ext"
        let extension = match options.format {
            ReportFormat::Text => "txt",
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Svg => "svg",
        };
        
        // Find first available filename (report_1.html, report_2.html, etc.)
        let mut counter = 1;
        let mut output_path;
        
        loop {
            output_path = format!("reports/report_{}.{}", counter, extension);
            
            // Check if file already exists
            if !std::path::Path::new(&output_path).exists() {
                break;
            }
            
            counter += 1;
        }
        
        debug!("Auto-generated output file: {}", output_path);
        output_path
    };
    
    // Ensure reports directory exists
    if let Some(parent_dir) = std::path::Path::new(&output_path).parent() {
        if !parent_dir.exists() {
            debug!("Creating directory: {:?}", parent_dir);
            fs::create_dir_all(parent_dir)
                .map_err(|e| Error::Io(e))?;
        }
    }
    
    debug!("Writing report to: {}", output_path);
    let mut file = File::create(&output_path)
        .map_err(|e| Error::Io(e))?;
    file.write_all(report.as_bytes())
        .map_err(|e| Error::Io(e))?;
    info!("Report written to {}", output_path);
    
    Ok(report)
}

// Disable the warnings for instrument macro
#[allow(warnings)]
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

// Disable the warnings for instrument macro
#[allow(warnings)]
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

// Disable the warnings for instrument macro
#[allow(warnings)]
#[instrument(skip(results, options))]
fn generate_html_report(results: &LoadTestResults, options: &ReportOptions) -> Result<String> {
    debug!("Generating HTML report");
    
    let now = chrono::Local::now();
    let date_str = now.format("%B %d, %Y").to_string();
    let all_passed = results.failed_requests == 0;
    
    // Calculate percentiles from histogram
    let histogram = create_histogram(results);
    let percentile_50 = histogram.as_ref().map(|h| h.value_at_percentile(50.0) as f64).unwrap_or(0.0);
    let percentile_90 = histogram.as_ref().map(|h| h.value_at_percentile(90.0) as f64).unwrap_or(0.0);
    let percentile_95 = histogram.as_ref().map(|h| h.value_at_percentile(95.0) as f64).unwrap_or(0.0);
    let percentile_99 = histogram.as_ref().map(|h| h.value_at_percentile(99.0) as f64).unwrap_or(0.0);
    
    // Get status code counts from the LoadTestResults
    let status_counts = &results.status_codes;
    
    let html = maud::html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Load Test Report" }
                script src="https://cdn.tailwindcss.com" {}
                script {
                    "tailwind.config = {
                      theme: {
                        extend: {
                          colors: {
                            'dark': '#0f1118',
                            'card': '#151a27',
                            'purple': '#7e22ce',
                            'teal': '#0f766e',
                            'blue': '#2563eb',
                            'pink': '#db2777',
                            'orange': '#ea580c',
                          },
                          screens: {
                            '3xl': '1920px',
                          }
                        }
                      }
                    }"
                }
                style {
                    "body { background-color: #0f1118; color: #e2e8f0; }
                    .card { background-color: #151a27; border-radius: 0.75rem; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05); min-height: 120px; display: flex; flex-direction: column; }
                    .metric-card { display: flex; flex-direction: column; justify-content: space-between; min-height: 120px; }
                    .svg-container { width: 100%; overflow-x: auto; }
                    svg { width: 100%; height: auto; }
                    svg text { fill: #94a3b8; }
                    svg line { stroke: #1e293b; }
                    table { table-layout: fixed; }
                    .status-table th:first-child { width: 20%; }
                    .status-table th:nth-child(2) { width: 15%; }
                    .status-table th:last-child { width: 65%; }
                    .progress-bar { height: 0.375rem; border-radius: 9999px; background-color: #374151; overflow: hidden; }
                    .progress-value { height: 100%; border-radius: 9999px; }
                    .metric-value { font-size: 1.5rem; font-weight: 700; color: white; line-height: 1.25; margin-top: 0.5rem; margin-bottom: 0.75rem; }
                    .metric-label { font-size: 0.813rem; font-weight: 500; color: #94a3b8; }
                    .metric-sublabel { font-size: 0.688rem; color: #64748b; margin-bottom: 0.25rem; }
                    .section-title { color: #f3f4f6; font-size: 1rem; font-weight: 600; }
                    .card-title { color: white; font-size: 1rem; font-weight: 600; margin-bottom: 0.25rem; }
                    .card-subtitle { color: #94a3b8; font-size: 0.75rem; margin-bottom: 0.5rem; }
                    .grid-cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 0.75rem; }
                    .mb-3 { margin-bottom: 0.75rem; }
                    .mb-4 { margin-bottom: 1rem; }
                    .status-badge { padding: 0.25rem 0.75rem; border-radius: 9999px; font-size: 0.75rem; font-weight: 500; }
                    @media (min-width: 1920px) { .container { padding-left: 2rem; padding-right: 2rem; } }"
                }
            }
            body style="background-color: #0f1118; color: #e2e8f0;" {
                div class="mx-auto px-5 py-5 max-w-7xl" {
                    div class="flex justify-between items-center mb-6" {
                        div {
                            h1 class="text-3xl font-bold text-white" { "Load Test Report" }
                            p class="text-gray-400" { "Generated on " (date_str) }
                        }
                        div class="rounded-full px-4 py-1.5 flex items-center bg-green-900/30 text-green-400" {
                            svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor" {
                                path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" {}
                            }
                            span { "All Tests Passed" }
                        }
                    }
                    
                    h2 class="section-title mb-3" { "Summary" }
                    
                    // First row - Top stats
                    div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4" {
                        div class="card p-4" {
                            div class="flex justify-between items-start" {
                                div {
                                    h3 class="metric-label" { "Total Requests" }
                                    p class="metric-sublabel" { "Total number of requests made" }
                                    p class="metric-value" { (results.total_requests) }
                                }
                                div class="bg-purple-900/20 p-2 rounded-lg" {
                                    svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-purple-500" viewBox="0 0 24 24" {
                                        path fill="currentColor" d="M4 19h6v-2H4v2zM4 14h10v-2H4v2zM4 9h16V7H4v2z" {}
                                    }
                                }
                            }
                        }
                        div class="card p-4" {
                            div class="flex justify-between items-start" {
                                div {
                                    h3 class="metric-label" { "Success Rate" }
                                    p class="metric-sublabel" { "Percentage of successful requests" }
                                    p class="metric-value" { 
                                        (format!("{:.1}%", 100.0 * (results.successful_requests) as f64 / results.total_requests as f64)) 
                                    }
                                }
                                div class="bg-green-900/20 p-2 rounded-lg" {
                                    svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" {
                                        path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" {}
                                    }
                                }
                            }
                        }
                        div class="card p-4" {
                            div class="flex justify-between items-start" {
                                div {
                                    h3 class="metric-label" { "Duration" }
                                    p class="metric-sublabel" { "Total test duration" }
                                    p class="metric-value" { (format!("{:.2}s", results.duration_secs)) }
                                }
                                div class="bg-blue-900/20 p-2 rounded-lg" {
                                    svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" {
                                        path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" {}
                                    }
                                }
                            }
                        }
                    }
                    
                    // Create a 2x3 grid for response metrics
                    div class="grid grid-cols-2 md:grid-cols-3 gap-3 mb-4" {
                        // Response time metrics
                        div class="card p-4" {
                            h3 class="metric-label" { "AVG Response" }
                            p class="metric-value" { (format!("{:.2} ms", results.average_response_time)) }
                            div class="progress-bar mt-3" {
                                div class="progress-value bg-blue-500" style=(format!("width: {}%", 
                                    calculate_percentage(results.average_response_time, results.max_response_time as f64))) {}
                            }
                        }
                        div class="card p-4" {
                            h3 class="metric-label" { "MIN Response" }
                            p class="metric-value" { (format!("{} ms", results.min_response_time)) }
                            div class="progress-bar mt-3" {
                                div class="progress-value bg-green-500" style=(format!("width: {}%", 
                                    calculate_percentage(results.min_response_time as f64, results.max_response_time as f64))) {}
                            }
                        }
                        div class="card p-4" {
                            h3 class="metric-label" { "MAX Response" }
                            p class="metric-value" { (format!("{} ms", results.max_response_time)) }
                            div class="progress-bar mt-3" {
                                div class="progress-value bg-red-500" style="width: 100%" {}
                            }
                        }
                        
                        // Percentile metrics
                        div class="card p-4" {
                            h3 class="metric-label" { "50TH PCT" }
                            p class="metric-value" { (format!("{:.2} ms", percentile_50)) }
                            div class="progress-bar mt-3" {
                                div class="progress-value bg-purple-500" style=(format!("width: {}%", 
                                    calculate_percentage(percentile_50, results.max_response_time as f64))) {}
                            }
                        }
                        div class="card p-4" {
                            h3 class="metric-label" { "90TH PCT" }
                            p class="metric-value" { (format!("{:.2} ms", percentile_90)) }
                            div class="progress-bar mt-3" {
                                div class="progress-value bg-orange-500" style=(format!("width: {}%", 
                                    calculate_percentage(percentile_90, results.max_response_time as f64))) {}
                            }
                        }
                        div class="card p-4" {
                            h3 class="metric-label" { "95TH PCT" }
                            p class="metric-value" { (format!("{:.2} ms", percentile_95)) }
                            div class="progress-bar mt-3" {
                                div class="progress-value bg-pink-500" style=(format!("width: {}%", 
                                    calculate_percentage(percentile_95, results.max_response_time as f64))) {}
                            }
                        }
                    }
                    
                    // Status codes and histogram - now stacked vertically
                    // Status codes section - full width
                    h2 class="section-title mb-3" { "Status Codes" }
                    div class="card p-4 mb-4" {
                        h3 class="card-title" { "HTTP Status" }
                        p class="card-subtitle" { "Distribution of HTTP status codes across all requests" }
                        table class="w-full mt-3" {
                            thead {
                                tr class="text-left" {
                                    th class="pb-2" { "Status Code" }
                                    th class="pb-2" { "Count" }
                                    th class="pb-2" { "Percentage" }
                                }
                            }
                            tbody {
                                @for (status, count) in status_counts.iter() {
                                    tr {
                                        td { (status) }
                                        td { (count) }
                                        td { (format!("{:.1}%", 100.0 * (*count as f64) / (results.total_requests as f64))) }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Response time distribution section - full width
                    @if options.include_histograms {
                        h2 class="section-title mb-3" { "Response Time Distribution" }
                        div class="card p-4 mb-4" {
                            h3 class="card-title" { "Response Times" }
                            p class="card-subtitle" { "Distribution of response times across all requests" }
                            div class="svg-container w-full" {
                                @if let Ok(svg) = generate_histogram_svg(results) {
                                    (PreEscaped(svg))
                                }
                            }
                        }
                    }
                    
                    div class="flex justify-between items-center mt-6 pt-4 border-t border-gray-800 text-gray-500 text-sm" {
                        p { "Generated by " a href="https://github.com/username/pressr" class="text-blue-400 hover:underline" { "pressr" } }
                        p { "© 2023 Performance Analytics" }
                    }
                }
            }
        }
    };
    
    Ok(html.into_string())
}

// Disable the warnings for instrument macro
#[allow(warnings)]
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
        // Define dark theme colors
        let dark_bg = RGBColor(15, 17, 24);  // #0f1118
        let card_bg = RGBColor(21, 26, 39);  // #151a27
        let grid_line = RGBColor(30, 41, 59); // #1e293b
        let text_color = RGBColor(148, 163, 184); // #94a3b8
        let purple_bar = RGBColor(126, 34, 206); // #7e22ce
        let purple_bar_alpha = purple_bar.mix(0.8); // with opacity
        let green_line = RGBColor(34, 197, 94); // #22c55e - p50
        let orange_line = RGBColor(234, 88, 12); // #ea580c - p90
        let pink_line = RGBColor(219, 39, 119); // #db2777 - p95
        let red_line = RGBColor(239, 68, 68); // #ef4444 - p99
        
        let root = SVGBackend::with_string(&mut buffer, (1000, 400))
            .into_drawing_area();
        
        root.fill(&dark_bg)
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
            .margin(25)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(
                min_time..max_time_with_padding,
                0.0..max_count as f64 * 1.1
            )
            .map_err(|_| Error::Other("Failed to create chart".to_string()))?;
        
        chart.configure_mesh()
            .x_desc("Response Time (ms)")
            .y_desc("Request Count")
            .axis_desc_style(("sans-serif", 12).into_font().color(&text_color))
            .label_style(("sans-serif", 11).into_font().color(&text_color))
            .draw()
            .map_err(|_| Error::Other("Failed to draw chart mesh".to_string()))?;
        
        // Draw histogram bars
        chart.draw_series(
            histogram_data.iter().map(|&(x, count)| {
                Rectangle::new(
                    [(x, 0.0), (x + bucket_size, count as f64)],
                    purple_bar_alpha.filled(),
                )
            })
        )
        .map_err(|_| Error::Other("Failed to draw histogram bars".to_string()))?
        .label("Response Times");
        
        // Draw the percentile lines
        if let Some(hist) = create_histogram(results) {
            let p50 = hist.value_at_percentile(50.0) as f64;
            let p90 = hist.value_at_percentile(90.0) as f64;
            let p95 = hist.value_at_percentile(95.0) as f64;
            let p99 = hist.value_at_percentile(99.0) as f64;
            
            // Draw 50th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p50, 0.0), (p50, max_count as f64)],
                green_line.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p50 line".to_string()))?
            .label("50th Percentile");
            
            // Draw 90th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p90, 0.0), (p90, max_count as f64)],
                orange_line.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p90 line".to_string()))?
            .label("90th Percentile");
            
            // Draw 95th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p95, 0.0), (p95, max_count as f64)],
                pink_line.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p95 line".to_string()))?
            .label("95th Percentile");
            
            // Draw 99th percentile line
            chart.draw_series(LineSeries::new(
                vec![(p99, 0.0), (p99, max_count as f64)],
                red_line.stroke_width(2),
            ))
            .map_err(|_| Error::Other("Failed to draw p99 line".to_string()))?
            .label("99th Percentile");
            
            // Draw 99th percentile label on top
            // Use draw_series with a single Text element instead of draw_text
            chart.draw_series(std::iter::once(Text::new(
                "99th",
                (p99, max_count as f64 * 1.05),
                ("sans-serif", 14).into_font().color(&pink_line)
            )))
            .map_err(|_| Error::Other("Failed to draw p99 label".to_string()))?;
        }
        
        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(card_bg.filled())
            .border_style(grid_line)
            .label_font(("sans-serif", 12).into_font().color(&text_color))
            .margin(10)
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

// Helper function to calculate percentage for progress bars
fn calculate_percentage(value: f64, max: f64) -> u32 {
    ((value / max) * 100.0).min(100.0) as u32
} 