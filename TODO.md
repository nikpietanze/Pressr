# pressr TODO List

This document tracks the development tasks for the `pressr` load testing tool.

**Project Structure:**

*   **Workspace Root:** `/home/nikp/dev/personal/pressr`
*   **Main CLI Crate:** `crates/pressr-cli`

**Development Tasks:**

1.  **[✓] Setup CLI Argument Parsing:**
    *   **Task:** Define command-line arguments (target URL, number of requests, concurrency, data file path, HTTP method, headers, etc.).
    *   **Tool:** Use the `clap` crate.
    *   **Files:**
        *   Modify: `crates/pressr-cli/src/main.rs`
        *   Modify: `crates/pressr-cli/Cargo.toml` (add `clap` dependency with "derive" feature).
    *   **Completed:** Implemented command-line arguments parsing with support for URL, HTTP method, request count, concurrency, headers, timeout, and output format.

2.  **[✓] Implement Basic HTTP Request:**
    *   **Task:** Send a single HTTP request to the specified URL using the given method and headers. Print basic response info (status code).
    *   **Tool:** Use the `reqwest` crate (likely async with `tokio`).
    *   **Files:**
        *   Modify: `crates/pressr-cli/src/main.rs`
        *   Modify: `crates/pressr-cli/Cargo.toml` (add `reqwest` with features like "json", "blocking" or async runtime like `tokio`).
    *   **Completed:** Implemented asynchronous HTTP requests with error handling, header support, and detailed response reporting (status code, time, size, and body).

3.  **[✓] Handle Input Data Loading:**
    *   **Task:** Read and parse the specified data file (start with JSON, maybe add YAML later). Define a structure to hold the data.
    *   **Tool:** Use `serde`, `serde_json`, `tokio::fs` (for async file reading).
    *   **Files:**
        *   Modify/Add: Potential new modules in `crates/pressr-cli/src/` (e.g., `data.rs`).
        *   Modify: `crates/pressr-cli/Cargo.toml` (add `serde`, `serde_json`, `tokio` with "fs" feature).
    *   **Completed:** Implemented a flexible data module with support for loading JSON files containing request bodies, headers, URL parameters, path variables, and variable data for randomization. Added error handling with `thiserror`.

4.  **[✓] Implement Concurrent Requests:**
    *   **Task:** Launch multiple requests concurrently based on user-defined level. Use data from the loaded file (randomly selected) if applicable.
    *   **Tool:** Use `tokio` tasks (`tokio::spawn`) and potentially `futures::stream`.
    *   **Files:**
        *   Modify: `crates/pressr-cli/src/main.rs` (or a new module, e.g., `runner.rs`).
    *   **Completed:** Created a runner module with support for concurrent request execution using `futures::stream::buffer_unordered`. Implemented request result tracking and aggregation into comprehensive load test results. Added a reporting module with both text and JSON output formats.

5.  **[✓] Collect and Store Results:**
    *   **Task:** Store results for each request: status code, response time, errors. Define a result structure.
    *   **Tool:** Standard Rust data structures (`Vec<RequestResult>`).
    *   **Files:**
        *   Modify/Add: New modules in `crates/pressr-cli/src/` (e.g., `results.rs`).
    *   **Completed:** Implemented `RequestResult` and `LoadTestResults` structs in the runner module to track individual request results and aggregate statistics such as success/failure counts, response times, and status code distribution.

6.  **[✓] Generate Basic Report:**
    *   **Task:** Analyze results and print a summary report (total requests, success/error count, basic timing, errors).
    *   **Tool:** Standard Rust printing (maybe `comfy-table` later).
    *   **Files:**
        *   Modify/Add: New module in `crates/pressr-cli/src/` (e.g., `report.rs`).
    *   **Completed:** Created a report module with support for both text and JSON report formats. Text reports include sections for summary, timing, status codes, and errors. JSON reports provide structured data for programmatic processing.

7.  **[✓] Refine Error Handling & Logging:**
    *   **Task:** Implement robust error handling (`Result`, `anyhow`/`thiserror`) and logging (`tracing`).
    *   **Tool:** `anyhow`/`thiserror`, `tracing`, `tracing-subscriber`.
    *   **Files:**
        *   Modify: Throughout `crates/pressr-cli`.
        *   Modify: `crates/pressr-cli/Cargo.toml`.
    *   **Completed:** Added centralized error handling in a dedicated `error.rs` module using `thiserror`. Implemented structured logging with `tracing` and `tracing-subscriber` throughout the application. Added verbose mode with `-v/--verbose` flag for detailed logging. Improved error reporting and context in all modules.

8.  **[✓] Core Library - Extract Reusable Logic:**
    *   **Task:** Extract reusable core functionality into a separate library crate that can be used by different frontends.
    *   **Tool:** Rust workspace, cargo, refactoring.
    *   **Files:**
        *   Add: `crates/pressr-core/` directory and required files.
        *   Modify: `crates/pressr-cli/` to use the core library.
        *   Modify: Root `Cargo.toml` to include the new crate in the workspace.
    *   **Completed:** Created a new `pressr-core` library crate that contains all the reusable logic: data models, error handling, HTTP request runner, and results processing. Refactored the CLI to use the core library instead of its own implementation, removing duplicate code. Updated the workspace configuration to include both crates.

9.  **[✓] Advanced Reporting:**
    *   **Task:** Enhance report generation with histograms, save reports to files (JSON, HTML), and add more detailed statistics.
    *   **Tool:** Use `plotters` for histograms, file I/O for saving, HTML templates for web reports.
    *   **Files:**
        *   Modify: `crates/pressr-core/src/result.rs` (add more statistics).
        *   Add: `crates/pressr-core/src/report.rs` (core report generation).
        *   Modify: `crates/pressr-cli/src/report.rs` (CLI-specific reporting).
        *   Modify: `crates/pressr-cli/src/main.rs` (add CLI flags for report options).
    *   **Completed:** Enhanced report generation with interactive visualizations using Chart.js, added advanced statistics (throughput, response time distribution, percentiles, and data transfer metrics). Implemented multiple report format support with a modular HTML template system. Added SVG histograms with percentile lines. Enabled custom output directory and self-contained report options.

**Refactoring & Cleanup:**

10. **[ ] Optimize Histogram Calculation:**
    *   **Task:** Calculate the `hdrhistogram::Histogram` once before report generation and pass it to relevant functions to avoid redundant computations.
    *   **Files:**
        *   Modify: `crates/pressr-core/src/report.rs`
        *   Modify: `crates/pressr-cli/src/main.rs` (potentially)

11. **[ ] Refactor Report Path Generation:**
    *   **Task:** Extract the logic for determining the output file path (handling directories, auto-generation) into a dedicated helper function within `report.rs`.
    *   **Files:**
        *   Modify: `crates/pressr-core/src/report.rs`

**Future Considerations:**

*   **[ ] Alternative Frontends:** TUI, GUI (Tauri), Wasm web interface. 