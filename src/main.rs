mod logging;
mod renderer;
mod ocr;
mod cli;
mod errors;
mod input;
mod xfa;

use clap::Parser;
use cli::{Cli, XfaMode, Mode};
use errors::CrabError;
use input::InputSource;
use renderer::Renderer;
use std::process;
use std::time::Instant;
use std::io::Write; // For flushing stdout

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}

fn run() -> Result<(), CrabError> {
    let args = Cli::parse();
    
    // Initialize logging
    logging::init(args.verbose);

    // Validate DPI
    if (args.mode == Mode::Ocr || args.mode == Mode::Hybrid) && (args.dpi < 72 || args.dpi > 600) {
        return Err(CrabError::Cli(format!(
            "DPI must be between 72 and 600. Got: {}",
            args.dpi
        )));
    }

    // Handle Input
    let input = InputSource::new(args.input)?;
    
    if args.verbose {
        match &input {
            InputSource::File(p) => eprintln!("Mode: File({:?})", p),
            InputSource::StdinBytes(b) => eprintln!("Mode: StdinBytes({} bytes)", b.len()),
            InputSource::TempFile(f) => eprintln!("Mode: TempFile({:?})", f.path()),
        }
        eprintln!("Config: lang='{}', dpi={}, xfa={:?}, mode={:?}, range='{}', timeout={}", 
            args.lang, args.dpi, args.xfa, args.mode, args.range, args.timeout);
    }
    
    // Initialize Renderer
    let renderer = Renderer::new()?;
    if args.verbose {
        eprintln!("Renderer initialized.");
    }

    let mut _tmp_file_handle: Option<tempfile::NamedTempFile> = None;
    let final_path = match &input {
        InputSource::File(p) => p.clone(),
        InputSource::TempFile(f) => f.path().to_path_buf(),
        InputSource::StdinBytes(b) => {
             let mut t = tempfile::NamedTempFile::new()?;
             t.write_all(b)?;
             let p = t.path().to_path_buf();
             _tmp_file_handle = Some(t);
             p
        }
    };

    let mut doc = renderer.open(&final_path)?;
    let page_count = renderer.page_count(&doc)?;
    
    if args.verbose {
        eprintln!("Opened document: {:?} ({} pages)", final_path, page_count);
    }
    
    // XFA Extraction
    if args.xfa != XfaMode::Off {
        if let Some(xml) = renderer.extract_xfa(&doc) {
            println!("--- XFA DATA START ---");
            
            match args.xfa {
                XfaMode::Off => {}, 
                XfaMode::Raw => print!("{}", xml),
                XfaMode::Full | XfaMode::Clean => {
                    let data_only = args.xfa == XfaMode::Clean;
                    match xfa::xfa_xml_to_json(&xml, data_only) {
                        Ok(json) => print!("{}", json),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse XFA content to structured JSON: {}", e);
                            eprintln!("Fallback: Outputting raw XFA XML.");
                            print!("{}", xml);
                        }
                    }
                }
            }
            println!("\n--- XFA DATA END ---");
            println!(); // Blank line between sections
        }
    }

    // Parse Range
    let pages_to_process = cli::parse_range(&args.range, page_count as usize)
        .map_err(|e| CrabError::Cli(format!("Invalid range: {}", e)))?;
    
    if args.verbose {
        eprintln!("Processing {} pages: {:?}", pages_to_process.len(), pages_to_process);
    }

    // Initialize OCR if needed
    let ocr = if args.mode == Mode::Ocr || args.mode == Mode::Hybrid {
        let ocr_instance = ocr::Ocr::new(&args.lang)?;
        if args.verbose {
            eprintln!("OCR initialized with lang '{}'.", args.lang);
        }
        Some(ocr_instance)
    } else {
        None
    };

    // Execution Loop
    let start_time = Instant::now();
    let mut timed_out = false;

    for &page_idx in &pages_to_process {
        // Timeout handling
        if args.timeout > 0 && start_time.elapsed().as_secs() > args.timeout {
             timed_out = true;
             break;
        }

        println!("--- PAGE {} START ---", page_idx + 1);
        println!(); // Blank line

        // Text Layer (Hybrid or Text modes)
        if args.mode == Mode::Hybrid || args.mode == Mode::Text {
            println!("--- TEXT LAYER START ---");
            match renderer.extract_text(&doc, page_idx as i32) {
                Ok(text) => print!("{}", text),
                Err(e) => eprintln!("Warning: Failed to extract text from page {}: {}", page_idx, e),
            }
            // The text output may contain newlines if the PDF structure suggests them.
            println!("--- TEXT LAYER END ---");
            println!(); // Blank line
        }

        // OCR Layer (Hybrid or Ocr modes)
        if let Some(ocr_engine) = &ocr {
             println!("--- OCR LAYER START ---");
             // Render
             let mut pix = renderer.render_page(&doc, page_idx as i32, args.dpi as i32)?;
             // Recognize
             let text = ocr_engine.recognize(&pix, &renderer, args.dpi as i32)?;
             print!("{}", text);
             // Cleanup pix
             pix.drop_with(&renderer);
             println!("--- OCR LAYER END ---");
             println!(); // Blank line
        }

        println!("--- PAGE {} END ---", page_idx + 1);
        println!(); // Blank line between pages or after page
    }
    
    // Clean up document
    doc.drop_with(&renderer);
    
    if timed_out {
        std::io::stdout().flush().ok();
        return Err(CrabError::Timeout);
    }
    
    Ok(())
}

