use pressr_core::LoadTestResults;
use serde::Serialize;
use std::collections::BTreeMap;
use tracing::{debug, info, instrument};

/// Formats for reports
#[derive(Debug)]
pub enum ReportFormat {
    /// Plain text report
    Text,
    /// JSON report
    Json,
}

/// Generate a report from the load test results
#[instrument(skip(results))]
pub fn generate_report(results: &LoadTestResults, format: ReportFormat) -> String {
    info!("Generating {} report for load test with {} requests", 
        format_name(&format), results.total_requests);
    
    match format {
        ReportFormat::Text => generate_text_report(results),
        ReportFormat::Json => generate_json_report(results),
    }
}

/// Helper to get format name as string
fn format_name(format: &ReportFormat) -> &'static str {
    match format {
        ReportFormat::Text => "text",
        ReportFormat::Json => "json",
    }
}

/// Generate a plain text report
#[instrument(skip(results))]
fn generate_text_report(results: &LoadTestResults) -> String {
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
    
    debug!("Text report generated ({} chars)", report.len());
    report
}

/// Generate a JSON report
#[instrument(skip(results))]
fn generate_json_report(results: &LoadTestResults) -> String {
    debug!("Generating JSON report");
    // Create a serializable version of the results
    #[derive(Serialize)]
    struct JsonReport {
        completed_requests: usize,
        successful_requests: usize,
        failed_requests: usize,
        total_duration_secs: f64,
        avg_duration_ms: f64,
        min_duration_ms: u128,
        max_duration_ms: u128,
        success_rate: f64,
        failure_rate: f64,
        status_codes: BTreeMap<u16, usize>,
        error_counts: BTreeMap<String, usize>,
    }
    
    // Convert status_codes HashMap to BTreeMap for deterministic ordering
    let status_codes: BTreeMap<_, _> = results.status_codes.iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    
    // Convert errors HashMap to BTreeMap for deterministic ordering
    let error_counts: BTreeMap<_, _> = results.errors.iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    
    let report = JsonReport {
        completed_requests: results.total_requests,
        successful_requests: results.successful_requests,
        failed_requests: results.failed_requests,
        total_duration_secs: results.duration_secs,
        avg_duration_ms: results.average_response_time,
        min_duration_ms: results.min_response_time,
        max_duration_ms: results.max_response_time,
        success_rate: percentage(results.successful_requests, results.total_requests) / 100.0,
        failure_rate: percentage(results.failed_requests, results.total_requests) / 100.0,
        status_codes,
        error_counts,
    };
    
    let json = serde_json::to_string_pretty(&report)
        .unwrap_or_else(|e| {
            debug!("Error generating JSON report: {}", e);
            String::from("Error generating JSON report")
        });
    
    debug!("JSON report generated ({} chars)", json.len());
    json
}

/// Calculate percentage
fn percentage(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
} 