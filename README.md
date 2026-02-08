# CrabOCR

**CrabOCR** is a high-performance, PDF-to-text converter designed specifically for LLM ingestion pipelines. It rund locally, uses **MuPDF** for rendering and **Tesseract** for optical character recognition, and produces UTF-8 text output from scanned documents or complex PDF layouts. 

## Context

CrabOCR idea came after repeatedly hitting PDF extraction failures in **automation workflows** like n8n or Power Automate. Due to the complexity of the PDF format and the million features it can hide, traditional text extraction is inherently fragile. For LLM pipelines, consistent extraction matters more than perfect text; modern language models excel at interpreting noisy OCR output, but the flow will fail completely when a PDF can't be processed.

CrabOCR is built to be:
*   **Stateless & Pipe-friendly**: Reads from stdin, writes to stdout. Perfect for containerized environments and Unix pipelines.
*   **Self-Contained**: Statically Linked and distribued as a single, dependency-free binary. Processes everything locally without external API calls or cloud services. No usage limits, no network required.
*   **Fast**: Uses direct C-API bindings to MuPDF and Tesseract, avoiding shell-out overhead.
*   **Consistent**: No Surprises, it renders all PDFs to images before OCR. Lower quality than text extraction (when text layers exist), but works consistently on any PDF, scanned, e-signed, encrypted, or malformed.

## Features

*   **PDF to Text**: Converts scanned and native PDFs to plain text.
*   **Standard Input/Output**: Seamless integration with other tools (e.g., `cat doc.pdf | crabocr | llm-ingest`).
*   **Zero Runtime Dependencies**: The static binary runs on any modern Linux distro. Should build on Windows (not tested).
*   **Configurable**: Control OCR language and rendering DPI via CLI arguments.

## Installation

### Static Binary (Recommended for Linux)

You can download the latest static linux binary from the [Releases page](https://github.com/wmahfoudh/crabocr/releases). Alternatively, you can build it yourself either locally or using Docker (see [Building from Source](#building-from-source)).

### Docker Image

A Dockerfile is provided to build a minimal, statically linked binary.

```bash
# Build the image
docker build -t crabocr .

# Run directly
docker run -i --rm crabocr < document.pdf
```

## Usage

CrabOCR is designed to be simple.

### Basic Usage

**File Input:**
```bash
./crabocr document.pdf
```

**Standard Input (Pipe):**
```bash
cat document.pdf | ./crabocr
```

### Options

*   `--lang <lang>`: ISO 639-3 language code (default: `eng`).
*   `--dpi <dpi>`: Rendering resolution in Dots Per Inch (default: `300`). Higher DPI improves accuracy for small text but is slower.
*   `--verbose`: Enable verbose logging to stderr.

**Example:**
```bash
./crabocr --lang fra --dpi 400 invoice.pdf > invoice.txt
```

### Language Data (Traineddata)

CrabOCR requires Tesseract trained data files (`.traineddata`).

**Where does it look?**
The program searches for a `tessdata` directory in the following order:
1.  **`TESSDATA_PREFIX` Environment Variable**:
    *   **Linux/macOS**:
        ```bash
        export TESSDATA_PREFIX=/path/to/your/tessdata_folder
        ```
    *   **Windows (PowerShell)**:
        ```powershell
        $env:TESSDATA_PREFIX = "C:\path\to\tessdata_folder"
        ```
2.  **Relative to Binary**: It looks for a `tessdata` folder in the same directory as the `crabocr` executable.
3.  **Current Working Directory**: It looks for a `tessdata` folder in your current directory.

**Adding Languages:**
1.  Download the desired language file (e.g., `fra.traineddata` for French) from the [tessdata_best](https://github.com/tesseract-ocr/tessdata_best) repository.
2.  Place it in your `tessdata` folder.
3.  Run with `--lang fra`.

## Building from Source

### Prerequisites
*   Rust (latest stable)
*   C/C++ Compiler (`gcc` or `clang`)
*   `cmake`
*   `git`
*   Build tools (`make`, `pkg-config`)

### Local Build (Linux)

This will compile MuPDF and Tesseract from source and link them statically.

```bash
# Clone the repository
git clone https://github.com/wmahfoudh/crabocr.git
cd crabocr

# Build in release mode
cargo build --release

# Binary location
ls target/release/crabocr
```

### Static Build (Alpine/Musl)

To produce a portable binary that runs on any Linux distribution, use the provided `Dockerfile`.

```bash
# 1. Build the builder image
docker build -t crabocr-builder .

# 2. Extract the binary
id=$(docker create crabocr-builder)
docker cp $id:/app/target/x86_64-unknown-linux-musl/release/crabocr .
docker rm -v $id

# 3. Verify
./crabocr --help
```

### Compiling on Windows

Since this project relies on `make` and C-dependencies that are complex to build natively on Windows, **we strongly recommend using WSL2 (Windows Subsystem for Linux)** or Docker.

**Using WSL2:**
1.  Install WSL2 (`wsl --install`).
2.  Open your Ubuntu/Debian terminal.
3.  Follow the **Local Build (Linux)** instructions above.


## License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**.

**Note:** If you use CrabOCR as part of a network service, you must make the source code available.

## Acknowledgements

*   [MuPDF](https://mupdf.com/) - PDF Rendering
*   [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) - Optical Character Recognition
*   [Leptonica](http://www.leptonica.org/) - Image Processing
