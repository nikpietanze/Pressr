# pressr

`pressr` is a lightweight HTTP load testing tool written in Rust, designed to benchmark web servers and APIs.

## Features

- **Simple CLI interface**: Easy to use with minimal setup
- **Multiple HTTP methods support**: GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH
- **Concurrent requests**: Test with configurable concurrency levels
- **Detailed reports**: Get comprehensive metrics in stdout, JSON, HTML, or SVG
- **Request timeout**: Configure request timeouts to avoid hanging tests
- **Custom headers**: Add custom HTTP headers to your requests
- **POST data**: Send data with POST requests from a file
- **Graphical User Interface**: Desktop application with simple visual interface

## Installation

### From Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/pressr.git
cd pressr
```

2. Build the CLI tool:
```bash
cargo build --release -p pressr-cli
```

3. Build the GUI application:
```bash
cargo tauri build -p pressr-gui
```

The CLI binary will be available at `target/release/pressr-cli` and the GUI application will be in the `target/release/bundle` directory.

### Prebuilt Binaries

Download prebuilt binaries from the [Releases](https://github.com/yourusername/pressr/releases) page.

## Usage

### Command Line

```bash
pressr-cli -u <URL> [OPTIONS]
```

Example:
```bash
pressr-cli -u https://example.com -m get -r 100 -c 10 --output html --detailed
```

### Graphical User Interface

1. Launch the application:
```bash
# If installed
pressr-gui

# Or directly from the build directory
./target/release/pressr-gui
```

2. In the GUI:
   - Enter the target URL
   - Configure request parameters (method, concurrency, number of requests)
   - Select output format
   - Click "Start Test" to begin load testing
   - View results in real-time within the application
   - Export reports in various formats

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
