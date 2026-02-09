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

    /// XFA-only mode. Extract XFA XML data and exit without OCR.
    #[arg(short = 'x', long)]
    pub xfa: bool,
}
