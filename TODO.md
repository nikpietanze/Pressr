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

4.  **[ ] Implement Concurrent Requests:**
    *   **Task:** Launch multiple requests concurrently based on user-defined level. Use data from the loaded file (randomly selected) if applicable.
    *   **Tool:** Use `tokio` tasks (`tokio::spawn`) and potentially `futures::stream`.
    *   **Files:**
        *   Modify: `crates/pressr-cli/src/main.rs` (or a new module, e.g., `runner.rs`).

5.  **[ ] Collect and Store Results:**
    *   **Task:** Store results for each request: status code, response time, errors. Define a result structure.
    *   **Tool:** Standard Rust data structures (`Vec<RequestResult>`).
    *   **Files:**
        *   Modify/Add: New modules in `crates/pressr-cli/src/` (e.g., `results.rs`).

6.  **[ ] Generate Basic Report:**
    *   **Task:** Analyze results and print a summary report (total requests, success/error count, basic timing, errors).
    *   **Tool:** Standard Rust printing (maybe `comfy-table` later).
    *   **Files:**
        *   Modify/Add: New module in `crates/pressr-cli/src/` (e.g., `report.rs`).

7.  **[ ] Refine Error Handling & Logging:**
    *   **Task:** Implement robust error handling (`Result`, `anyhow`/`thiserror`) and logging (`tracing`).
    *   **Tool:** `anyhow`/`thiserror`, `tracing`, `tracing-subscriber`.
    *   **Files:**
        *   Modify: Throughout `crates/pressr-cli`.
        *   Modify: `crates/pressr-cli/Cargo.toml`.

**Future Considerations:**

*   **[ ] Core Library:** Extract reusable logic into `crates/pressr-core`.
*   **[ ] Advanced Reporting:** Histograms, saving reports (JSON, HTML).
*   **[ ] Alternative Frontends:** TUI, GUI (Tauri), Wasm web interface. 