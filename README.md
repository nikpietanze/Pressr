<p align="center">
  <img src="./pressr-logo.png" width="160" alt="pressr logo" />
</p>

<h1 align="center">pressr</h1>

<p align="center">
  ⚡️ Load test APIs and applications with thousands of concurrent requests. Designed for modern devs. Built for open-source scalability.
</p>

---

## ✨ Features

- 🚀 High-concurrency request engine
- 📊 Tracks response times, errors, and full output
- 🧠 Minimal, scriptable interface (CLI-first)
- 🧪 Perfect for CI pipelines, stress tests, and perf validation
- 🌐 Web-ready architecture (future: browser-based load tests)
- 🔧 Cross-compiled binaries: Windows `.exe`, macOS `.dmg`, Linux
- 🛠 Open source, minimal dependencies

---

## 🔧 Example Usage

```sh
pressr run \
  --url https://api.example.com/endpoint \
  --method POST \
  --body '{"key": "value"}' \
  --concurrency 1000 \
  --duration 30s

