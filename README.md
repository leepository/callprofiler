# callprofiler

A Python profiling library that generates interactive HTML call graph visualizations. The core processing engine is written in Rust for minimal overhead.

## Features

- **Simple decorator API** - Add `@profile` to any function to start profiling
- **Async support** - Works with both sync and async functions
- **Interactive HTML reports** - Collapsible call trees with timing data
- **Slowest path highlighting** - The bottleneck function is highlighted in red
- **Library detection** - Distinguishes user code from stdlib and third-party packages
- **Rust-powered** - Event processing and HTML generation run in Rust via PyO3
- **Cross-platform** - Pre-built wheels for Linux, macOS, and Windows

## Installation

```bash
pip install callprofiler
```

### Build from source

Requires [Rust](https://rustup.rs/) and [Maturin](https://github.com/PyO3/maturin):

```bash
pip install maturin
maturin develop --release
```

## Usage

### Basic

```python
from callprofiler import profile

@profile
def my_function():
    prepare_data()
    process_data()

@profile(output_dir="custom_dir")
def another_function():
    ...
```

### Async functions

```python
from callprofiler import profile

@profile
async def fetch_and_process():
    data = await fetch_data()
    return await process_data(data)
```

When the decorated function executes, an HTML report is saved to the `.profile/` directory (or the specified `output_dir`):

```
[callprofiler] Report saved to .profile/my_function_20260211_153520.html
```

Open the HTML file in a browser to explore the call graph.

## Report contents

Each report includes:

- **Total duration** of the profiled function
- **Slowest function** identification with duration
- **Function count** across the call tree
- **Call tree** with per-function timing, source location, and start/end timestamps
- **Library badges** for external calls (e.g., `json`, `time`, `builtins`)

## How it works

1. The `@profile` decorator installs a `sys.setprofile` hook that captures `call`, `return`, `c_call`, and `c_return` events
2. Events are collected as Python dicts with function name, module, filename, line number, nanosecond timestamp, and library classification
3. After execution, events are passed to the Rust extension which builds a call tree and generates an HTML report
4. The report is written to disk as a self-contained HTML file with embedded CSS and JavaScript

## Project structure

```
callprofiler/
├── src/                       # Rust source
│   ├── lib.rs                 # PyO3 module entry point
│   ├── call_node.rs           # Call tree construction
│   └── reporter.rs            # HTML report generation
├── python/callprofiler/       # Python package
│   ├── __init__.py
│   └── profiler.py            # Decorator and sys.setprofile hook
├── docs/
│   └── USAGE.md               # Usage guide (Korean)
├── .github/workflows/
│   └── CI.yml                 # CI/CD pipeline
├── Cargo.toml
└── pyproject.toml
```

## Supported platforms

Pre-built wheels are available for:

| OS | Architectures |
|----|---------------|
| Linux (glibc) | x86_64, x86, aarch64, armv7, s390x, ppc64le |
| Linux (musl) | x86_64, x86, aarch64, armv7 |
| macOS | x86_64, aarch64 |
| Windows | x64, x86, aarch64 |

## Requirements

- Python >= 3.9
- Rust (for building from source)
