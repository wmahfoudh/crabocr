# CrabOCR

**CrabOCR** is a high-performance, stateless PDF-to-text converter designed specifically for LLM ingestion pipelines. It leverages **MuPDF** for precise rendering and **Tesseract** for optical character recognition, producing clean UTF-8 text output from scanned documents and complex PDF layouts.

Unlike traditional tools, CrabOCR is built to be:
*   **Stateless & Pipe-friendly**: Reads from stdin, writes to stdout. Perfect for containerized environments and Unix pipelines.
*   **Statically Linked**: Distribued as a single, dependency-free binary (Linux/Musl).
*   **Fast**: Uses direct C-API bindings to MuPDF and Tesseract, avoiding shell-out overhead.

## Features

*   **PDF to Text**: Converts scanned and native PDFs to plain text.
*   **Standard Input/Output**: Seamless integration with other tools (e.g., `cat doc.pdf | crabocr | llm-ingest`).
*   **Zero Runtime Dependencies**: The static binary runs on any modern Linux distro (Arch, Alpine, Debian, Ubuntu, etc.) without installing separate libraries.
*   **Configurable**: Control OCR language and rendering DPI via CLI arguments.

## Installation

### Static Binary (Recommended for Linux)

You can download the latest static binary from the [Releases page](https://github.com/wmahfoudh/crabocr/releases) or build it yourself using Docker.

### Docker Image

We provide a Dockerfile to build a minimal, statically linked binary.

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
1.  **`TESSDATA_PREFIX` Environment Variable**: If set, it checks this path first.
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

## License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**.
This is required because we statically link against **MuPDF**, which is AGPL licensed.

**Note:** If you use this software as part of a network service, you must make the source code available to users of that service.

## Acknowledgements

*   [MuPDF](https://mupdf.com/) - PDF Rendering
*   [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) - Optical Character Recognition
*   [Leptonica](http://www.leptonica.org/) - Image Processing
