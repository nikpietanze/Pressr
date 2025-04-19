use crate::runner::LoadTestResults;
use serde::Serialize;
use std::collections::BTreeMap;

/// Formats for reports
pub enum ReportFormat {
    /// Plain text report
    Text,
    /// JSON report
    Json,
}

/// Generate a report from the load test results
pub fn generate_report(results: &LoadTestResults, format: ReportFormat) -> String {
    match format {
        ReportFormat::Text => generate_text_report(results),
        ReportFormat::Json => generate_json_report(results),
    }
}

/// Generate a plain text report
fn generate_text_report(results: &LoadTestResults) -> String {
    let mut report = String::new();
    
    // Header
    report.push_str(&format!("LOAD TEST REPORT - {}\n", results.url));
    report.push_str(&format!("HTTP Method: {}\n", results.method));
    report.push_str(&format!("Requests: {}\n", results.completed_requests));
    report.push_str("\n");
    
    // Summary
    report.push_str("SUMMARY\n");
    report.push_str(&format!("Total requests:     {}\n", results.completed_requests));
    report.push_str(&format!("Successful (2xx):   {} ({:.1}%)\n", 
        results.successful_requests, 
        percentage(results.successful_requests, results.completed_requests)
    ));
    report.push_str(&format!("Failed:            {} ({:.1}%)\n", 
        results.failed_requests, 
        percentage(results.failed_requests, results.completed_requests)
    ));
    report.push_str("\n");
    
    // Timing
    report.push_str("TIMING\n");
    report.push_str(&format!("Total duration:     {} ms\n", results.total_duration_ms));
    report.push_str(&format!("Average:            {} ms\n", results.avg_duration_ms));
    report.push_str(&format!("Minimum:            {} ms\n", results.min_duration_ms));
    report.push_str(&format!("Maximum:            {} ms\n", results.max_duration_ms));
    report.push_str("\n");
    
    // Status codes
    if !results.status_code_counts.is_empty() {
        report.push_str("STATUS CODES\n");
        
        // Sort status codes for consistent output
        let mut sorted_status_codes: Vec<_> = results.status_code_counts.iter().collect();
        sorted_status_codes.sort_by_key(|&(code, _)| *code);
        
        for (code, count) in sorted_status_codes {
            let percent = percentage(*count, results.completed_requests);
            report.push_str(&format!("{}: {} ({:.1}%)\n", code, count, percent));
        }
        report.push_str("\n");
    }
    
    // Error summary
    let error_count = results.results.iter().filter(|r| r.error.is_some()).count();
    if error_count > 0 {
        report.push_str("ERRORS\n");
        let mut error_counts = BTreeMap::new();
        
        for result in &results.results {
            if let Some(error) = &result.error {
                *error_counts.entry(error).or_insert(0) += 1;
            }
        }
        
        for (error, count) in error_counts {
            let percent = percentage(count, results.completed_requests);
            report.push_str(&format!("{}: {} ({:.1}%)\n", error, count, percent));
        }
        report.push_str("\n");
    }
    
    report
}

/// Generate a JSON report
fn generate_json_report(results: &LoadTestResults) -> String {
    // Create a serializable version of the results
    #[derive(Serialize)]
    struct JsonReport<'a> {
        url: &'a str,
        method: &'a str,
        completed_requests: usize,
        successful_requests: usize,
        failed_requests: usize,
        total_duration_ms: u128,
        avg_duration_ms: u128,
        min_duration_ms: u128,
        max_duration_ms: u128,
        success_rate: f64,
        failure_rate: f64,
        status_codes: BTreeMap<u16, usize>,
        error_counts: BTreeMap<String, usize>,
    }
    
    // Count errors
    let mut error_counts = BTreeMap::new();
    for result in &results.results {
        if let Some(error) = &result.error {
            *error_counts.entry(error.clone()).or_insert(0) += 1;
        }
    }
    
    // Convert status_code_counts HashMap to BTreeMap for deterministic ordering
    let status_codes: BTreeMap<_, _> = results.status_code_counts.iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    
    let report = JsonReport {
        url: &results.url,
        method: &results.method,
        completed_requests: results.completed_requests,
        successful_requests: results.successful_requests,
        failed_requests: results.failed_requests,
        total_duration_ms: results.total_duration_ms,
        avg_duration_ms: results.avg_duration_ms,
        min_duration_ms: results.min_duration_ms,
        max_duration_ms: results.max_duration_ms,
        success_rate: percentage(results.successful_requests, results.completed_requests) / 100.0,
        failure_rate: percentage(results.failed_requests, results.completed_requests) / 100.0,
        status_codes,
        error_counts,
    };
    
    serde_json::to_string_pretty(&report).unwrap_or_else(|_| String::from("Error generating JSON report"))
}

/// Calculate percentage
fn percentage(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
} 