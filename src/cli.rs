use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Input PDF file. If not provided, reads from STDIN.
    #[arg(value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Tesseract language code(s).
    #[arg(short, long, default_value = "eng")]
    pub lang: String,

    /// DPI for rasterization.
    #[arg(short, long, default_value_t = 300)]
    pub dpi: u32,

    /// Enable verbose logging to STDERR.
    #[arg(short, long)]
    pub verbose: bool,

    /// XFA extraction mode.
    #[arg(short = 'x', long, value_enum, default_value_t = XfaMode::Clean)]
    pub xfa: XfaMode,

    /// OCR mode.
    #[arg(short = 'o', long, value_enum, default_value_t = OcrMode::On)]
    pub ocr: OcrMode,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum XfaMode {
    /// Skip XFA processing.
    Off,
    /// Output original XFA XML.
    Raw,
    /// Output full parsed JSON.
    Full,
    /// Output cleaned form-data JSON.
    Clean,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum OcrMode {
    /// Perform rendering and OCR.
    On,
    /// Skip rendering and OCR.
    Off,
}
