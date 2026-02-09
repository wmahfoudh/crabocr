mod logging;
mod renderer;
mod ocr;
mod cli;
mod errors;
mod input;

use clap::Parser;
use cli::Cli;
use errors::CrabError;
use input::InputSource;
use renderer::Renderer;
use std::process;


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

    // Validate DPI (only needed for OCR mode)
    if !args.xfa && (args.dpi < 72 || args.dpi > 600) {
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
        eprintln!("Config: lang='{}', dpi={}, xfa_only={}", args.lang, args.dpi, args.xfa);
    }
    
    // Initialize Renderer (needed for both XFA extraction and OCR)
    let renderer = Renderer::new()?;
    if args.verbose {
        eprintln!("Renderer initialized.");
    }

    let mut _tmp_file_handle: Option<tempfile::NamedTempFile> = None;
    let final_path = match &input {
        InputSource::File(p) => p.clone(),
        InputSource::TempFile(f) => f.path().to_path_buf(),
        InputSource::StdinBytes(b) => {
             use std::io::Write;
             let mut t = tempfile::NamedTempFile::new()?;
             t.write_all(b)?;
             let p = t.path().to_path_buf();
             _tmp_file_handle = Some(t);
             p
        }
    };

    let mut doc = renderer.open(&final_path)?;
    
    if args.verbose {
        let page_count = renderer.page_count(&doc)?;
        eprintln!("Opened document: {:?} ({} pages)", final_path, page_count);
    }
    
    // XFA Extraction (Pre-OCR)
    let xfa_data = renderer.extract_xfa(&doc);
    
    if args.xfa {
        // XFA-only mode: print raw XML and exit
        if let Some(xml) = xfa_data {
            print!("{}", xml);
        }
        // Exit without OCR (no output if no XFA)
        doc.drop_with(&renderer);
        return Ok(());
    }
    
    // Hybrid mode (default): print XFA with delimiters if present
    if let Some(ref xml) = xfa_data {
        println!("--- XFA DATA START ---");
        print!("{}", xml);
        println!("--- XFA DATA END ---");
    }
    
    // Initialize OCR (deferred to avoid loading Tesseract in XFA-only mode)
    let ocr = ocr::Ocr::new(&args.lang)?;
    if args.verbose {
        eprintln!("OCR initialized with lang '{}'.", args.lang);
    }
    
    let page_count = renderer.page_count(&doc)?;
    
    // Render and OCR pages
    for i in 0..page_count {
        // 1. Render
        let mut pix = renderer.render_page(&doc, i, args.dpi as i32)?;
        
        // Safety check (optional, but good practice)
        if args.verbose {
             // eprintln!("Page {}: {}x{} ...", i, pix.width(&renderer), pix.height(&renderer));
        }

        // 2. OCR
        let text = ocr.recognize(&pix, &renderer)?;
        
        // 3. Output
        print!("{}", text);
        
        // 4. Separator (between pages)
        if i < page_count - 1 {
            print!("\n\x0c\n"); // \n\f\n
        }
        
        // Clean up page resources
        pix.drop_with(&renderer);
    }
    
    // Clean up document
    doc.drop_with(&renderer);
    
    Ok(())
}

