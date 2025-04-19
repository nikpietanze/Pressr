use pressr_core::{LoadTestResults, ReportFormat as CoreReportFormat, ReportOptions};
use tracing::{debug, info};

/// Formats for reports
#[derive(Debug)]
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

impl From<ReportFormat> for CoreReportFormat {
    fn from(format: ReportFormat) -> Self {
        match format {
            ReportFormat::Text => CoreReportFormat::Text,
            ReportFormat::Json => CoreReportFormat::Json,
            ReportFormat::Html => CoreReportFormat::Html,
            ReportFormat::Svg => CoreReportFormat::Svg,
        }
    }
}

/// Generate a report from the load test results
pub fn generate_report(results: &LoadTestResults, format: ReportFormat) -> String {
    info!("Generating {} report for load test with {} requests", 
          format_name(&format), results.total_requests);
    
    // Create report options
    let options = ReportOptions {
        format: format.into(),
        output_file: None,
        include_histograms: false,
        include_details: false,
    };
    
    // Generate the report using the core library function
    match pressr_core::generate_report(results, &options) {
        Ok(report) => report,
        Err(e) => {
            debug!("Error generating report: {}", e);
            format!("Error generating report: {}", e)
        }
    }
}

/// Helper to get format name as string
fn format_name(format: &ReportFormat) -> &'static str {
    match format {
        ReportFormat::Text => "text",
        ReportFormat::Json => "json",
        ReportFormat::Html => "html",
        ReportFormat::Svg => "svg",
    }
} 