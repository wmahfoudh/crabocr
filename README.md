# CrabOCR

**CrabOCR** is a high-performance PDF-to-text converter designed for LLM ingestion, RAG pipelines, and stateless FaaS runtimes (AWS Lambda, Cloud Run).

Standard PDF parsers often fail on scanned pages, encrypted files, or dynamic Adobe XFA forms. CrabOCR bypasses these issues by rendering the PDF into upscaled images before performing OCR. It also includes a dedicated engine to extract and structure hidden XFA form data into cleaned JSON.

## Core Capabilities

* **OCR**: Uses **MuPDF** for high-fidelity rendering and **Tesseract** for text recognition. Successfully handles scanned documents, hand-signed forms, and complex layouts.
* **XFA Data Extraction**: Automatically detects Adobe XFA forms. It extracts raw XML and converts it into a cleaned JSON structure, stripping system metadata and lookup bloat.
* **Stateless & Pipe-Friendly**: Reads from `stdin` and writes to `stdout`. Perfect for containerized environments and Unix-style automation pipelines.

## Installation

### 1. Static Binary (Linux)
Download the pre-compiled static binary from the [Releases](https://github.com/wmahfoudh/crabocr/releases) page. This version is self-contained and runs on any Linux distribution (including Ubuntu, CentOS, and Alpine) without requiring external libraries aside from the language files.

### 2. Docker (Static Build)
To produce your own portable, statically linked binary using Alpine Linux:

```bash
# 1. Build the builder image
docker build -t crabocr-builder .

# 2. Extract the binary to your current directory
id=$(docker create crabocr-builder)
docker cp $id:/app/target/x86_64-unknown-linux-musl/release/crabocr .
docker rm -v $id

# 3. Verify the build
./crabocr --version

```

### 3. Build from Source

If you prefer to build locally, ensure you have the Rust toolchain, `cmake`, `clang`, and `pkg-config` installed.

```bash
# Clone and build
git clone [https://github.com/wmahfoudh/crabocr.git](https://github.com/wmahfoudh/crabocr.git)
cd crabocr
cargo build --release

# The binary will be located at:
# ./target/release/crabocr

```

## Language Support (Traineddata)

CrabOCR requires Tesseract `.traineddata` files. It searches for a `tessdata` folder in this order:

1. **`TESSDATA_PREFIX`**: Environment variable path.
2. **Binary Location**: A `tessdata/` folder in the same directory as the executable.
3. **Current Directory**: A `tessdata/` folder in your current working directory.

**Adding Languages:**

1. Download the required `.traineddata` (e.g., `fra.traineddata`) from the [tessdata_best](https://github.com/tesseract-ocr/tessdata_best) repository.
2. Place it in one of the locations above and run with `-l fra`.

## Usage & Options

```text
Usage: crabocr [OPTIONS] [FILE]

Arguments:
  [FILE]  Input PDF file. If not provided, reads from STDIN

Options:
  -l, --lang <LANG>  Tesseract language code(s) [default: eng]
  -d, --dpi <DPI>    DPI for rasterization [default: 300]
  -v, --verbose      Enable verbose logging to STDERR
  -x, --xfa <XFA>    XFA extraction mode [default: clean] [values: off, raw, full, clean]
  -o, --ocr <OCR>    OCR mode [default: on] [values: on, off]
  -h, --help         Print help
  -V, --version      Print version

```

### Example Workflows

**1. Data-Only Extraction (Sub-second)**
Instantly extract form values as JSON and skip heavy OCR processing.

```bash
./crabocr document.pdf -o off -x clean > data.json

```

**2. Multi-Language OCR with Verbose Logging**
Process a document in English and French while watching the progress in `stderr`.

```bash
./crabocr -l eng+fra -v document.pdf

```

**3. No-Op Protection**
If both modes are disabled (`-x off -o off`), the program exits with code `1` and prints an error to `stderr` to prevent empty files in automation.

## Formatting Note

When both XFA and OCR are enabled, output is separated by a clear delimiter and a blank line for easy programmatic splitting:

```text
--- XFA DATA START ---
{ "field": "value" }
--- XFA DATA END ---

[OCR Text starts here...]

```

## License

**AGPL-3.0**. If you modify this tool or host it as a service, you must make your source code available.
