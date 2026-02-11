# CrabOCR

**CrabOCR** is a high-performance PDF and Image-to-text converter designed for LLM ingestion, RAG pipelines, and stateless FaaS runtimes (AWS Lambda, Cloud Run).

Standard PDF parsers often fail on scanned pages, encrypted files, or dynamic Adobe XFA forms. CrabOCR bypasses these issues by offering a hybrid approach: it can extract embedded text directly for speed, and/or render pages into upscaled images for OCR to handle complex layouts and scans. It also includes a dedicated engine to extract and structure Adobe XFA form data into cleaned JSON.

## Core Capabilities

*   **Hybrid Extraction**:
    *   **Text Mode**: Instantly extracts embedded text layers using **MuPDF** (fast, perfect for digital-native PDFs).
    *   **OCR Mode**: Renders pages to high-res images and uses **Tesseract** to recognize text (robust for scans and complex layouts).
    *   **Hybrid Mode**: Extracts both layers sequentially, providing the ultimate context for RAG pipelines.
*   **Image Support**: Natively processes standalone image files (JPG, PNG, TIFF) in addition to PDFs.
*   **XFA Data Extraction**: Automatically detects Adobe XFA forms. It extracts raw XML and converts it into a cleaned JSON structure, stripping system metadata and lookup bloat.
*   **Stateless & Pipe-Friendly**: Reads from `stdin` and writes to `stdout` with strict delimiter formatting. Perfect for containerized environments and Unix-style automation pipelines.

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
git clone https://github.com/wmahfoudh/crabocr.git
cd crabocr
cargo build --release

# The binary will be located at:
# ./target/release/crabocr
```

## Language Support (Traineddata)

CrabOCR requires Tesseract `.traineddata` files. It searches for a `tessdata` folder in this order:

1.  **`TESSDATA_PREFIX`**: Environment variable path.
2.  **Binary Location**: A `tessdata/` folder in the same directory as the executable.
3.  **Current Directory**: A `tessdata/` folder in your current working directory.

**Adding Languages:**

1.  Download the required `.traineddata` (e.g., `fra.traineddata`) from the [tessdata_best](https://github.com/tesseract-ocr/tessdata_best) repository.
2.  Place it in one of the locations above and run with `-l fra`.

## Usage & Options

```text
Usage: crabocr [OPTIONS] [FILE]

Arguments:
  [FILE]  Input PDF or Image file. If not provided, reads from STDIN

Options:
  -m, --mode <MODE>     Extraction mode [default: hybrid] [values: hybrid, text, ocr]
  -l, --lang <LANG>     Tesseract language code(s) [default: eng]
  -r, --range <RNG>     Page range to process (e.g., "1-5", "1,3,10"). Default is all pages.
  -t, --timeout <SEC>   Global timeout in seconds. Exits with code 2 if exceeded.
  -d, --dpi <DPI>       DPI for rasterization (used in ocr/hybrid modes) [default: 300]
  -v, --verbose         Enable verbose logging to STDERR
  -x, --xfa <XFA>       XFA extraction mode [default: clean] [values: off, raw, full, clean]
  -h, --help            Print help
  -V, --version         Print version
```

### Example Workflows

**1. Fast Digital Text Extraction**
Skip the heavy OCR process and only extract the embedded text layer and XFA data.

```bash
./crabocr document.pdf -m text -x clean
```

**2. RAG Pipeline (Hybrid + Page Range)**
Process only the first 5 pages, extracting both the text layer and the visual OCR layer for maximum context.

```bash
./crabocr large_report.pdf -m hybrid -r 1-5
```

**3. Image Processing**
Run OCR on a scanned image file.

```bash
./crabocr receipt.jpg -m ocr
```

**4. Safety Timeout**
Enforce a hard limit on processing time. If the file takes longer than 60 seconds, the program flushes the current buffer and exits with code `2`.

```bash
./crabocr complex_scan.pdf -t 60
```

## Output Formatting

CrabOCR outputs a strict hierarchical structure designed for programmatic parsing. Sections are separated by clear delimiters and blank lines.

**Structure Overview:**

```text
--- XFA DATA START ---
{ "field": "value" }
--- XFA DATA END ---

--- PAGE 1 START ---

--- TEXT LAYER START ---
[Text extracted via MuPDF]
--- TEXT LAYER END ---

--- OCR LAYER START ---
[Text extracted via Tesseract]
--- OCR LAYER END ---

--- PAGE 1 END ---
```

*   **XFA Section**: Printed once at the start (if `-x` is enabled).
*   **Text Layer**: Appears if `-m text` or `-m hybrid` is used.
*   **OCR Layer**: Appears if `-m ocr` or `-m hybrid` is used.

## License

**AGPL-3.0**. If you modify this tool or host it as a service, you must make your source code available.
