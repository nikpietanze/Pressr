use crate::result::{LoadTestResults, RequestResult};
use crate::error::{Error, Result};
use hdrhistogram::Histogram;
use plotters::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use tracing::{debug, info, instrument, warn};
use serde::Serialize;
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
    
    /// Custom output directory (None for default 'reports/')
    pub output_dir: Option<String>,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            format: ReportFormat::Html,
            output_file: None,
            include_histograms: true,
            include_details: false,
            output_dir: None,
        }
    }
}

const HTML_TEMPLATE: &str = include_str!("../templates/report.html");

/// Preprocessed data for report generation
pub struct PreprocessedData<'a> {
    /// Reference to the original results
    pub results: &'a LoadTestResults,
    /// Calculated histogram (if available)
    pub histogram: Option<Histogram<u64>>,
}

impl<'a> PreprocessedData<'a> {
    /// Create a new PreprocessedData instance with calculated histogram and percentiles
    pub fn new(results: &'a LoadTestResults) -> Self {
        // Calculate histogram once
        let histogram = create_histogram(results);
        
        Self {
            results,
            histogram,
        }
    }
    
    /// Get percentile value
    pub fn percentile(&self, p: f64) -> Option<f64> {
        if let Some(hist) = &self.histogram {
            Some(hist.value_at_percentile(p) as f64)
        } else {
            None
        }
    }
}

// Disable the warnings for instrument macro as it's an environmental issue
#[allow(warnings)]
#[instrument(skip(results, options))]
pub fn generate_report(results: &LoadTestResults, options: &ReportOptions) -> Result<String> {
    info!("Generating {:?} report for load test with {} requests", 
          options.format, results.total_requests);
    
    // Preprocess data (histogram, percentiles) once
    let preprocessed = PreprocessedData::new(results);
    
    let report = match options.format {
        ReportFormat::Text => generate_text_report(&preprocessed, options),
        ReportFormat::Json => generate_json_report(&preprocessed, options),
        ReportFormat::Html => generate_html_report(&preprocessed, options),
        ReportFormat::Svg => generate_histogram_svg(&preprocessed)
    }?;
    
    // Get the output path (using the helper function)
    let output_path = get_output_path(options)?;
    
    // For HTML reports, copy the logo file to the reports directory
    if options.format == ReportFormat::Html {
        copy_logo_file(options)?;
    }
    
    debug!("Writing report to: {}", output_path);
    let mut file = File::create(&output_path)
        .map_err(|e| Error::Io(e))?;
    file.write_all(report.as_bytes())
        .map_err(|e| Error::Io(e))?;
    info!("Report written to {}", output_path);
    
    Ok(report)
}

/// Get output file path based on options
fn get_output_path(options: &ReportOptions) -> Result<String> {
    // Get the project root directory (or working directory)
    let project_root = std::env::current_dir()
        .map_err(|e| Error::Io(e))?;
    
    // Get the output directory (user-specified or default 'reports/')
    let base_dir = if let Some(dir) = &options.output_dir {
        if std::path::Path::new(dir).is_absolute() {
            dir.clone()
        } else {
            // Relative path - prepend project root
            project_root.join(dir).to_string_lossy().to_string()
        }
    } else {
        // Default to 'reports/' in project root
        project_root.join("reports").to_string_lossy().to_string()
    };
    
    // Generate output file path (user-specified or auto-generated)
    let output_path = if let Some(path) = &options.output_file {
        debug!("Using user-specified output file: {}", path);
        
        // Get just the filename component, ignoring any directory parts
        let filename = std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        
        // Place in the specified output directory
        format!("{}/{}", base_dir, filename)
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
            output_path = format!("{}/report_{}.{}", base_dir, counter, extension);
            
            // Check if file already exists
            if !std::path::Path::new(&output_path).exists() {
                break;
            }
            
            counter += 1;
        }
        
        debug!("Auto-generated output file: {}", output_path);
        output_path
    };
    
    // Ensure output directory exists
    if let Some(parent_dir) = std::path::Path::new(&output_path).parent() {
        if !parent_dir.exists() {
            debug!("Creating directory: {:?}", parent_dir);
            fs::create_dir_all(parent_dir)
                .map_err(|e| Error::Io(e))?;
        }
    }
    
    Ok(output_path)
}

/// Copy logo file for HTML reports
fn copy_logo_file(options: &ReportOptions) -> Result<()> {
    // Get the project root directory
    let project_root = std::env::current_dir()
        .map_err(|e| Error::Io(e))?;
    
    // Get the output directory (user-specified or default 'reports/')
    let base_dir = if let Some(dir) = &options.output_dir {
        if std::path::Path::new(dir).is_absolute() {
            dir.clone()
        } else {
            // Relative path - prepend project root
            project_root.join(dir).to_string_lossy().to_string()
        }
    } else {
        // Default to 'reports/' in project root
        project_root.join("reports").to_string_lossy().to_string()
    };
    
    // Get the path to the logo file (in the assets directory relative to project root)
    let logo_src_path = project_root.join("assets/images/pressr-logo.png").to_string_lossy().to_string();
    let logo_dest_path = format!("{}/pressr-logo.png", base_dir);
    
    // Only copy if the source exists
    if std::path::Path::new(&logo_src_path).exists() {
        debug!("Copying logo file to reports directory");
        if let Err(e) = fs::copy(&logo_src_path, &logo_dest_path) {
            warn!("Failed to copy logo file: {}", e);
            // Don't fail the report generation if logo copy fails
        } else {
            debug!("Logo file copied to {}", logo_dest_path);
        }
    } else {
        warn!("Logo file not found at {}", logo_src_path);
    }
    
    Ok(())
}

// Disable the warnings for instrument macro
#[allow(warnings)]
#[instrument(skip(preprocessed, options))]
fn generate_text_report(preprocessed: &PreprocessedData, options: &ReportOptions) -> Result<String> {
    debug!("Generating text report");
    let results = preprocessed.results;
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
    if let Some(p50) = preprocessed.percentile(50.0) {
        report.push_str(&format!("50th percentile:     {:.2} ms\n", p50));
        if let Some(p90) = preprocessed.percentile(90.0) {
            report.push_str(&format!("90th percentile:     {:.2} ms\n", p90));
        }
        if let Some(p95) = preprocessed.percentile(95.0) {
            report.push_str(&format!("95th percentile:     {:.2} ms\n", p95));
        }
        if let Some(p99) = preprocessed.percentile(99.0) {
            report.push_str(&format!("99th percentile:     {:.2} ms\n", p99));
        }
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
                let error_text = result.error
                    .as_deref()
                    .unwrap_or("Unknown")
                    .replace("HTTP Error: ", "");
                
                report.push_str(&format!("Failed, Error: {}, ", error_text));
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
#[instrument(skip(preprocessed, options))]
fn generate_json_report(preprocessed: &PreprocessedData, options: &ReportOptions) -> Result<String> {
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
        
        // New fields for enhanced reporting
        throughput: f64,
        response_time_std_dev: f64,
        total_data_transferred: Option<usize>,
        transfer_rate: Option<f64>,
        
        #[serde(skip_serializing_if = "Option::is_none")]
        request_details: Option<&'a Vec<RequestResult>>,
        
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        response_time_distribution: &'a HashMap<String, usize>,
    }
    
    // Calculate percentiles if histograms are enabled
    let percentiles = if options.include_histograms {
        if let Some(hist) = create_histogram(preprocessed.results) {
            let mut map = HashMap::new();
            
            map.insert("p50".to_string(), hist.value_at_percentile(50.0) as f64);
            map.insert("p75".to_string(), hist.value_at_percentile(75.0) as f64);
            map.insert("p90".to_string(), hist.value_at_percentile(90.0) as f64);
            map.insert("p95".to_string(), hist.value_at_percentile(95.0) as f64);
            map.insert("p99".to_string(), hist.value_at_percentile(99.0) as f64);
            map.insert("p999".to_string(), hist.value_at_percentile(99.9) as f64);
            
            Some(map)
        } else {
            None
        }
    } else {
        None
    };
    
    // Convert status_codes HashMap to BTreeMap for deterministic ordering
    let status_codes = preprocessed.results.status_codes.clone();
    
    // Convert errors HashMap to BTreeMap for deterministic ordering
    let error_counts = preprocessed.results.errors.clone();
    
    // Calculate percentages
    let success_rate = percentage(preprocessed.results.successful_requests, preprocessed.results.total_requests);
    let failure_rate = percentage(preprocessed.results.failed_requests, preprocessed.results.total_requests);
    
    // Optional detailed results
    let request_details = if options.include_details {
        Some(&preprocessed.results.requests)
    } else {
        None
    };
    
    // Create the JSON report
    let report = JsonReport {
        completed_requests: preprocessed.results.total_requests,
        successful_requests: preprocessed.results.successful_requests,
        failed_requests: preprocessed.results.failed_requests,
        total_duration_secs: preprocessed.results.duration_secs,
        avg_duration_ms: preprocessed.results.average_response_time,
        min_duration_ms: preprocessed.results.min_response_time,
        max_duration_ms: preprocessed.results.max_response_time,
        percentiles,
        success_rate,
        failure_rate,
        status_codes,
        error_counts,
        
        // New fields
        throughput: preprocessed.results.throughput,
        response_time_std_dev: preprocessed.results.response_time_std_dev,
        total_data_transferred: preprocessed.results.total_data_transferred,
        transfer_rate: preprocessed.results.transfer_rate,
        response_time_distribution: &preprocessed.results.response_time_distribution,
        
        request_details,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| Error::Json(e))?;
    
    debug!("JSON report generated ({} chars)", json.len());
    Ok(json)
}

/// Generate an enhanced HTML report with interactive charts
fn generate_html_report(preprocessed: &PreprocessedData, options: &ReportOptions) -> Result<String> {
    debug!("Generating enhanced HTML report");
    
    // Create chart data in JSON format for the JavaScript charts
    let chart_data = serde_json::json!({
        "summary": {
            "total": preprocessed.results.total_requests,
            "successful": preprocessed.results.successful_requests,
            "failed": preprocessed.results.failed_requests,
            "duration": preprocessed.results.duration_secs
        },
        "timing": {
            "average": preprocessed.results.average_response_time,
            "min": preprocessed.results.min_response_time,
            "max": preprocessed.results.max_response_time,
            "stdDev": preprocessed.results.response_time_std_dev,
            "throughput": preprocessed.results.throughput,
            "transferRate": preprocessed.results.transfer_rate
        },
        "distribution": {
            "responseTimes": preprocessed.results.response_time_distribution,
            "statusCodes": preprocessed.results.status_codes
        },
        "percentiles": create_percentile_data(preprocessed.results),
        "errors": preprocessed.results.errors
    });
    
    // Format the chart data as JSON string for embedding in the HTML
    let chart_data_json = serde_json::to_string(&chart_data)
        .map_err(|e| Error::Serialization(e))?;
        
    // Start with our HTML template
    let template = HTML_TEMPLATE.replace(
        "/* CHART_DATA_PLACEHOLDER */", 
        &format!("const chartData = {};", chart_data_json)
    );
    
    // Add metadata
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let metadata = format!(
        "Test Date: {}",
        timestamp
    );
    
    let html = template.replace("<!-- METADATA_PLACEHOLDER -->", &metadata);
    
    // Generate and embed SVG histograms if requested
    let html = if options.include_histograms {
        let response_time_histogram = generate_histogram_svg_embedded(preprocessed.results, "Response Time Distribution (ms)")?;
        html.replace("<!-- HISTOGRAM_PLACEHOLDER -->", &response_time_histogram)
    } else {
        html.replace("<!-- HISTOGRAM_PLACEHOLDER -->", "")
    };
    
    // Add detailed request information if requested
    let html = if options.include_details {
        let mut details_html = String::from("<h3>Request Details</h3>");
        
        // Add filter controls
        details_html.push_str(r#"
        <div class="filter-controls">
            <div class="filter-group">
                <label for="status-filter">Status Code:</label>
                <select id="status-filter">
                    <option value="all">All</option>
                    <option value="200">200 (Success)</option>
                    <option value="404">404 (Not Found)</option>
                    <option value="500">500 (Server Error)</option>
                </select>
            </div>
            <div class="filter-group">
                <label for="result-filter">Result:</label>
                <select id="result-filter">
                    <option value="all">All</option>
                    <option value="success">Success</option>
                    <option value="error">Error</option>
                </select>
            </div>
            <button id="reset-filters" class="filter-button">Reset</button>
        </div>
        "#);
        
        // Add table with ID for JavaScript manipulation
        details_html.push_str(r#"<div class="table-container"><table class="details-table" id="request-details-table">"#);
        details_html.push_str("<thead><tr><th>#</th><th>Status</th><th>Time (ms)</th><th>Size (bytes)</th><th>Result</th></tr></thead><tbody>");
        
        for (i, result) in preprocessed.results.requests.iter().enumerate() {
            let status = result.status.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
            let size = result.response_size.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
            let result_text = if result.success {
                "Success".to_string()
            } else {
                let error_text = result.error
                    .as_deref()
                    .unwrap_or("Unknown")
                    .replace("HTTP Error: ", "");
                
                format!("Error: {}", error_text)
            };
            
            details_html.push_str(&format!(
                r#"<tr data-status="{}" data-result="{}"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class="{}">{}</td></tr>"#,
                status,
                if result.success { "success" } else { "error" },
                i + 1,
                status,
                result.response_time,
                size,
                if result.success { "success" } else { "error" },
                result_text
            ));
            
            // If we have errors, ensure they're also included in the chart data
            if !result.success && result.error.is_some() {
                // Errors are already added to the LoadTestResults struct when it's created
                // in LoadTestResults::new() in result.rs, so we don't need to do anything extra here
            }
        }
        
        details_html.push_str("</tbody></table></div>");
        
        // Add pagination controls
        details_html.push_str(r#"
        <div class="pagination-controls">
            <button id="prev-page" class="pagination-button">&laquo; Previous</button>
            <span id="page-info">Page <span id="current-page">1</span> of <span id="total-pages">1</span></span>
            <button id="next-page" class="pagination-button">Next &raquo;</button>
            <select id="page-size">
                <option value="10">10 per page</option>
                <option value="20" selected>20 per page</option>
                <option value="50">50 per page</option>
                <option value="100">100 per page</option>
            </select>
        </div>
        "#);
        
        html.replace("<!-- DETAILS_PLACEHOLDER -->", &details_html)
    } else {
        html.replace("<!-- DETAILS_PLACEHOLDER -->", "")
    };
    
    Ok(html)
}

/// Create percentile data for charts
fn create_percentile_data(results: &LoadTestResults) -> HashMap<String, f64> {
    let mut percentiles = HashMap::new();
    
    if let Some(hist) = create_histogram(results) {
        // Add standard percentiles
        percentiles.insert("p50".to_string(), hist.value_at_percentile(50.0) as f64);
        percentiles.insert("p75".to_string(), hist.value_at_percentile(75.0) as f64);
        percentiles.insert("p90".to_string(), hist.value_at_percentile(90.0) as f64);
        percentiles.insert("p95".to_string(), hist.value_at_percentile(95.0) as f64);
        percentiles.insert("p99".to_string(), hist.value_at_percentile(99.0) as f64);
        percentiles.insert("p999".to_string(), hist.value_at_percentile(99.9) as f64);
    }
    
    percentiles
}

// Disable the warnings for instrument macro
#[allow(warnings)]
#[instrument(skip(preprocessed))]
fn generate_histogram_svg(preprocessed: &PreprocessedData) -> Result<String> {
    debug!("Generating SVG histogram");
    
    // Create a buffer for the SVG
    let mut buffer = String::new();
    
    // Extract response times
    let response_times: Vec<f64> = preprocessed.results.requests.iter()
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
        if let Some(hist) = &preprocessed.histogram {
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

/// Generate standalone SVG histogram for embedding in HTML reports
fn generate_histogram_svg_embedded(results: &LoadTestResults, title: &str) -> Result<String> {
    debug!("Generating embedded SVG histogram");
    
    // Create a histogram from response times
    let hist = match create_histogram(results) {
        Some(h) => h,
        None => return Ok("No data available for histogram".to_string()),
    };
    
    // Dimensions for the SVG
    let width = 800u32;
    let height = 400u32;
    
    // Create an in-memory SVG buffer as a String
    let mut buffer = String::new();
    {
        let root = SVGBackend::with_string(&mut buffer, (width, height))
            .into_drawing_area();
            
        root.fill(&WHITE)
            .map_err(|e| Error::Plotting(format!("Failed to fill plot background: {}", e)))?;
            
        let max_time = results.max_response_time as f64;
        let min_time = results.min_response_time as f64;
        let range = max_time - min_time;
        
        // Create bins for the histogram
        let bin_count = 20;
        let bin_size = range / bin_count as f64;
        
        // Create and populate histogram data
        let mut hist_data = Vec::new();
        let p99 = hist.value_at_percentile(99.0) as f64;
        
        // Cap the x-axis at p99 to avoid outliers stretching the graph
        let max_x = p99 * 1.1;
        
        for i in 0..bin_count {
            let bin_start = min_time + (i as f64 * bin_size);
            let bin_end = bin_start + bin_size;
            let mid_point = (bin_start + bin_end) / 2.0;
            
            // Count values in this bin
            let count = results.requests.iter()
                .filter(|r| {
                    let t = r.response_time as f64;
                    t >= bin_start && t < bin_end
                })
                .count();
                
            if count > 0 {
                hist_data.push((mid_point, count as f64));
            }
        }
        
        let mut chart = ChartBuilder::on(&root)
            .caption(title, ("sans-serif", 20))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(0f64..max_x, 0f64..hist_data.iter().map(|(_, c)| *c).fold(0.0, f64::max) * 1.1)
            .map_err(|e| Error::Plotting(format!("Failed to build chart: {}", e)))?;
            
        chart.configure_mesh()
            .x_desc("Response Time (ms)")
            .y_desc("Count")
            .draw()
            .map_err(|e| Error::Plotting(format!("Failed to draw chart mesh: {}", e)))?;
            
        // Draw the histogram bars
        chart.draw_series(
            hist_data.iter().map(|(x, y)| {
                let x = *x;
                let y = *y;
                Rectangle::new([(x - bin_size/2.0, 0.0), (x + bin_size/2.0, y)], BLUE.filled())
            })
        )
        .map_err(|e| Error::Plotting(format!("Failed to draw histogram bars: {}", e)))?;
        
        // Draw percentile lines
        let p50 = hist.value_at_percentile(50.0) as f64;
        let p90 = hist.value_at_percentile(90.0) as f64;
        
        let max_y = hist_data.iter().map(|(_, c)| *c).fold(0.0, f64::max);
        
        // Draw the median line (50th percentile)
        chart.draw_series(LineSeries::new(
            vec![(p50, 0.0), (p50, max_y)],
            &RED.mix(0.5),
        ))
        .map_err(|e| Error::Plotting(format!("Failed to draw median line: {}", e)))?
        .label("50th percentile")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));
        
        // Draw the 90th percentile line
        chart.draw_series(LineSeries::new(
            vec![(p90, 0.0), (p90, max_y)],
            &GREEN.mix(0.5),
        ))
        .map_err(|e| Error::Plotting(format!("Failed to draw p90 line: {}", e)))?
        .label("90th percentile")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));
        
        // Draw the 99th percentile line
        chart.draw_series(LineSeries::new(
            vec![(p99, 0.0), (p99, max_y)],
            &YELLOW.mix(0.5),
        ))
        .map_err(|e| Error::Plotting(format!("Failed to draw p99 line: {}", e)))?
        .label("99th percentile")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &YELLOW));
        
        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .map_err(|e| Error::Plotting(format!("Failed to draw legend: {}", e)))?;
            
        root.present()
            .map_err(|e| Error::Plotting(format!("Failed to render plot: {}", e)))?;
    }
    
    // Since we're using a String buffer, no conversion needed
    Ok(buffer)
}

/// Create a histogram from the response times
fn create_histogram(results: &LoadTestResults) -> Option<Histogram<u64>> {
    if results.requests.is_empty() {
        return None;
    }
    
    // Create histogram with 3 significant figures precision
    let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000, 3)
        .expect("Failed to create histogram with specified bounds");
    
    // Record response times (in milliseconds)
    for result in &results.requests {
        if result.success {
            hist.record(result.response_time as u64)
                .expect("Failed to record value in histogram");
        }
    }
    
    if hist.len() > 0 {
        Some(hist)
    } else {
        None
    }
}

/// Calculate percentage
fn percentage(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
} 