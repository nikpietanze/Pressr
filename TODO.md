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

10. **[✓] Optimize Histogram Calculation:**
    *   **Task:** Calculate the `hdrhistogram::Histogram` once before report generation and pass it to relevant functions to avoid redundant computations.
    *   **Files:**
        *   Modify: `crates/pressr-core/src/report.rs`
        *   Modify: `crates/pressr-cli/src/main.rs` (potentially)

11. **[✓] Refactor Report Path Generation:**
    *   **Task:** Extract the logic for determining the output file path (handling directories, auto-generation) into a dedicated helper function within `report.rs`.
    *   **Files:**
        *   Modify: `crates/pressr-core/src/report.rs`

**Tauri GUI Frontend:**

12. **[ ] Setup Tauri Project:**
    *   **Task:** Integrate Tauri into the existing Rust workspace. Create a new crate (e.g., `crates/pressr-gui`) for the Tauri application.
    *   **Tool:** `cargo tauri init`, `cargo`, Rust workspace management.
    *   **Files:**
        *   Modify: Root `Cargo.toml` (add new crate to workspace).
        *   Add: `crates/pressr-gui/` directory with Tauri structure (`src/`, `src-tauri/`, `tauri.conf.json`, `Cargo.toml`, etc.).
        *   Add: Frontend files (e.g., `index.html`, basic JS/CSS or framework setup).

13. **[ ] Basic UI Layout:**
    *   **Task:** Design and implement the initial user interface structure for inputting load test parameters (URL, method, requests, concurrency, headers, data file selection, timeout).
    *   **Tool:** HTML, CSS, potentially a simple JS framework (like vanilla JS, Svelte, or Vue).
    *   **Files:**
        *   Modify: Frontend files within `crates/pressr-gui/src/`.

14. **[ ] Tauri Backend Commands:**
    *   **Task:** Define Tauri commands in the Rust backend (`src-tauri/src/main.rs`) to handle interactions from the frontend, specifically to trigger a load test using `pressr-core`.
    *   **Tool:** Tauri API (`#[tauri::command]`), Rust.
    *   **Files:**
        *   Modify: `crates/pressr-gui/src-tauri/src/main.rs`.
        *   Modify: `crates/pressr-gui/src-tauri/Cargo.toml` (add `pressr-core` as a dependency).

15. **[ ] Frontend-Backend Communication:**
    *   **Task:** Implement JavaScript code in the frontend to gather input values and invoke the Tauri backend command to start the load test.
    *   **Tool:** Tauri JavaScript API (`invoke`).
    *   **Files:**
        *   Modify: Frontend JavaScript files within `crates/pressr-gui/src/`.

16. **[ ] Display Test Progress & Results:**
    *   **Task:** Show progress indication while the test is running. Display the summary results (`LoadTestResults`) in the UI after the test completes. Handle potential errors during the test run.
    *   **Tool:** Tauri Events or Commands for progress updates, frontend UI updates (JS/DOM manipulation).
    *   **Files:**
        *   Modify: `crates/pressr-gui/src-tauri/src/main.rs` (potentially emit events or return results).
        *   Modify: Frontend files within `crates/pressr-gui/src/`.

17. **[ ] Report Handling:**
    *   **Task:** Decide how to handle report generation. Options:
        *   A) Trigger report generation via backend command, save to disk, and provide a link/button to open the file.
        *   B) Display results directly in the Tauri UI, perhaps mimicking parts of the HTML report format.
    *   **Tool:** Tauri API (commands, events, filesystem access), frontend UI elements.
    *   **Files:**
        *   Modify: `crates/pressr-gui/src-tauri/src/main.rs`.
        *   Modify: Frontend files within `crates/pressr-gui/src/`.

18. **[ ] Packaging & Distribution:**
    *   **Task:** Configure `tauri.conf.json` and use Tauri CLI to build distributable application bundles.
    *   **Tool:** `cargo tauri build`.
    *   **Files:**
        *   Modify: `crates/pressr-gui/tauri.conf.json`. 