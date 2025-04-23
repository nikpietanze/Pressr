import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface FormData {
  url: string;
  method: string;
  requests: number;
  concurrency: number;
  timeout?: number;
  headers: { key: string; value: string }[];
}

interface TestResult {
  requestCount: number;
  successCount: number;
  failureCount: number;
  totalTime: number;
  averageTime: number;
  minTime: number;
  maxTime: number;
  throughput: number;
  successRate: number;
  statusCounts: Record<string, number>;
  errorCounts: Record<string, number>;
}

// Define the interface for the Rust backend response
interface BackendResponse {
  results: {
    request_count: number;
    success_count: number;
    failure_count: number;
    total_time: number;
    average_time: number;
    min_time: number;
    max_time: number;
    throughput: number;
    success_rate: number;
    status_counts: Record<string, number>;
    error_counts: Record<string, number>;
  };
}

function App() {
  const [formData, setFormData] = useState<FormData>({
    url: "https://httpbin.org/get",
    method: "GET",
    requests: 100,
    concurrency: 10,
    timeout: 30000,
    headers: [],
  });
  
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<TestResult | null>(null);
  const [rawResults, setRawResults] = useState<BackendResponse | null>(null);
  
  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData({
      ...formData,
      [name]: name === "requests" || name === "concurrency" || name === "timeout" 
        ? Number(value) 
        : value,
    });
  };
  
  const addHeader = () => {
    setFormData({
      ...formData,
      headers: [...formData.headers, { key: "", value: "" }],
    });
  };
  
  const removeHeader = (index: number) => {
    const newHeaders = [...formData.headers];
    newHeaders.splice(index, 1);
    setFormData({
      ...formData,
      headers: newHeaders,
    });
  };
  
  const updateHeader = (index: number, field: "key" | "value", value: string) => {
    const newHeaders = [...formData.headers];
    newHeaders[index][field] = value;
    setFormData({
      ...formData,
      headers: newHeaders,
    });
  };
  
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);
    setTestResults(null);
    setRawResults(null);
    
    try {
      // Convert headers array to object format expected by backend
      const headersObject: Record<string, string> = {};
      formData.headers.forEach(header => {
        if (header.key.trim()) {
          headersObject[header.key] = header.value;
        }
      });
      
      // Log the params we're sending to help debug
      const params = {
        url: formData.url,
        method: formData.method,
        requests: formData.requests,
        concurrency: formData.concurrency,
        timeout_ms: formData.timeout,
        headers: Object.keys(headersObject).length > 0 ? headersObject : null,
      };
      console.log("Sending params:", params);
      
      const result = await invoke<BackendResponse>("run_load_test", { params });
      console.log("Received result:", result);
      
      // Store the raw results for debugging
      setRawResults(result);
      
      // Safely extract fields with fallbacks for missing data
      if (!result?.results) {
        throw new Error("Invalid response format from server");
      }
      
      const r = result.results;
      setTestResults({
        requestCount: r.request_count ?? 0,
        successCount: r.success_count ?? 0,
        failureCount: r.failure_count ?? 0,
        totalTime: r.total_time ?? 0,
        averageTime: r.average_time ?? 0,
        minTime: r.min_time ?? 0,
        maxTime: r.max_time ?? 0,
        throughput: r.throughput ?? 0,
        successRate: r.success_rate ?? 0,
        statusCounts: r.status_counts ?? {},
        errorCounts: r.error_counts ?? {},
      });
    } catch (err) {
      console.error("Load test error:", err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };
  
  // Safe number formatter with fallback
  const formatNumber = (value: number | undefined | null, decimals = 2) => {
    if (value === undefined || value === null) return "N/A";
    return value.toFixed(decimals);
  };
  
  // Debug section
  const renderDebugInfo = () => {
    if (!rawResults) return null;
    
    return (
      <div className="debug-section">
        <details>
          <summary>Debug Information</summary>
          <pre>{JSON.stringify(rawResults, null, 2)}</pre>
        </details>
      </div>
    );
  };
  
  return (
    <div className="app-container">
      <header className="app-header">
        <h1>Pressr</h1>
        <p>HTTP Load Testing Tool</p>
      </header>
      
      <main className="main-content">
        <section className="config-section">
          <h2>Test Configuration</h2>
          
          <form onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="url">Target URL:</label>
              <input
                type="url"
                id="url"
                name="url"
                value={formData.url}
                onChange={handleChange}
                required
                className="form-control"
              />
            </div>
            
            <div className="form-group">
              <label htmlFor="method">HTTP Method:</label>
              <select
                id="method"
                name="method"
                value={formData.method}
                onChange={handleChange}
                className="form-control"
              >
                <option value="GET">GET</option>
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
                <option value="DELETE">DELETE</option>
                <option value="PATCH">PATCH</option>
                <option value="HEAD">HEAD</option>
                <option value="OPTIONS">OPTIONS</option>
              </select>
            </div>
            
            <div className="form-row">
              <div className="form-group">
                <label htmlFor="requests">Number of Requests:</label>
                <input
                  type="number"
                  id="requests"
                  name="requests"
                  min="1"
                  max="100000"
                  value={formData.requests}
                  onChange={handleChange}
                  required
                  className="form-control"
                />
              </div>
              
              <div className="form-group">
                <label htmlFor="concurrency">Concurrency Level:</label>
                <input
                  type="number"
                  id="concurrency"
                  name="concurrency"
                  min="1"
                  max="1000"
                  value={formData.concurrency}
                  onChange={handleChange}
                  required
                  className="form-control"
                />
              </div>
              
              <div className="form-group">
                <label htmlFor="timeout">Timeout (ms):</label>
                <input
                  type="number"
                  id="timeout"
                  name="timeout"
                  min="100"
                  max="60000"
                  value={formData.timeout}
                  onChange={handleChange}
                  className="form-control"
                />
              </div>
            </div>
            
            <div className="form-group">
              <label>HTTP Headers:</label>
              {formData.headers.map((header, index) => (
                <div key={index} className="header-row">
                  <input
                    type="text"
                    placeholder="Header name"
                    value={header.key}
                    onChange={(e) => updateHeader(index, "key", e.target.value)}
                    className="form-control header-key"
                  />
                  <input
                    type="text"
                    placeholder="Value"
                    value={header.value}
                    onChange={(e) => updateHeader(index, "value", e.target.value)}
                    className="form-control header-value"
                  />
                  <button
                    type="button"
                    onClick={() => removeHeader(index)}
                    className="btn btn-remove"
                  >
                    ✕
                  </button>
                </div>
              ))}
              <button
                type="button"
                onClick={addHeader}
                className="btn btn-add-header"
              >
                Add Header
              </button>
            </div>
            
            <div className="form-actions">
              <button
                type="submit"
                disabled={isLoading}
                className="btn btn-primary"
              >
                {isLoading ? "Running Test..." : "Start Load Test"}
              </button>
            </div>
          </form>
        </section>
        
        {error && (
          <section className="error-section">
            <h3>Error</h3>
            <div className="error-message">{error}</div>
          </section>
        )}
        
        {testResults && (
          <section className="results-section">
            <h2>Test Results</h2>
            
            <div className="results-summary">
              <div className="result-card">
                <h3>Requests</h3>
                <div className="result-value">{testResults.requestCount}</div>
              </div>
              
              <div className="result-card">
                <h3>Success Rate</h3>
                <div className="result-value">{formatNumber(testResults.successRate * 100)}%</div>
              </div>
              
              <div className="result-card">
                <h3>Avg Response</h3>
                <div className="result-value">{formatNumber(testResults.averageTime)} ms</div>
              </div>
              
              <div className="result-card">
                <h3>Throughput</h3>
                <div className="result-value">{formatNumber(testResults.throughput)} req/sec</div>
              </div>
            </div>
            
            <div className="results-detail">
              <div className="result-table">
                <h3>Response Times</h3>
                <table>
                  <tbody>
                    <tr>
                      <td>Min</td>
                      <td>{formatNumber(testResults.minTime)} ms</td>
                    </tr>
                    <tr>
                      <td>Max</td>
                      <td>{formatNumber(testResults.maxTime)} ms</td>
                    </tr>
                    <tr>
                      <td>Average</td>
                      <td>{formatNumber(testResults.averageTime)} ms</td>
                    </tr>
                    <tr>
                      <td>Total Time</td>
                      <td>{formatNumber(testResults.totalTime / 1000)} sec</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              
              <div className="result-table">
                <h3>Status Codes</h3>
                <table>
                  <tbody>
                    {Object.entries(testResults.statusCounts || {}).map(([code, count]) => (
                      <tr key={code}>
                        <td>{code}</td>
                        <td>{count}</td>
                      </tr>
                    ))}
                    {!Object.keys(testResults.statusCounts || {}).length && (
                      <tr><td colSpan={2}>No status codes collected</td></tr>
                    )}
                  </tbody>
                </table>
              </div>
              
              {Object.keys(testResults.errorCounts || {}).length > 0 && (
                <div className="result-table">
                  <h3>Errors</h3>
                  <table>
                    <tbody>
                      {Object.entries(testResults.errorCounts || {}).map(([error, count]) => (
                        <tr key={error}>
                          <td>{error}</td>
                          <td>{count}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </section>
        )}
        
        {renderDebugInfo()}
      </main>
    </div>
  );
}

export default App;
