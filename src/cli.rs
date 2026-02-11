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

    /// Extraction mode.
    #[arg(short = 'm', long, value_enum, default_value_t = Mode::Hybrid)]
    pub mode: Mode,

    /// Page range (e.g., "1-3,5,10"). Default is "all".
    #[arg(short, long, default_value = "all")]
    pub range: String,

    /// Timeout in seconds (default: 0, no timeout).
    #[arg(short, long, default_value_t = 0)]
    pub timeout: u64,
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
pub enum Mode {
    /// Extract text (MuPDF) then OCR (Tesseract).
    Hybrid,
    /// Extract text only (MuPDF).
    Text,
    /// Render and OCR only (Tesseract).
    Ocr,
}

pub fn parse_range(range_str: &str, max_pages: usize) -> anyhow::Result<Vec<usize>> {
    if range_str.eq_ignore_ascii_case("all") {
        return Ok((0..max_pages).collect());
    }

    let mut pages = std::collections::HashSet::new();

    for part in range_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((start, end)) = part.split_once('-') {
            let start: usize = start.trim().parse()?;
            let end: usize = end.trim().parse()?;
            // User input is 1-based, internal is 0-based
            for i in start..=end {
                if i > 0 && i <= max_pages {
                    pages.insert(i - 1);
                }
            }
        } else {
            let page: usize = part.parse()?;
            if page > 0 && page <= max_pages {
                pages.insert(page - 1);
            }
        }
    }

    let mut sorted_pages: Vec<usize> = pages.into_iter().collect();
    sorted_pages.sort();
    Ok(sorted_pages)
}
