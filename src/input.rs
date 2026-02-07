use std::io::{self, Read, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use crate::errors::CrabError;

const MAX_INMEM_PDF_BYTES: usize = 64 * 1024 * 1024; // 64 MiB

#[derive(Debug)]
pub enum InputSource {
    File(PathBuf),
    StdinBytes(Vec<u8>),
    TempFile(NamedTempFile),
}

impl InputSource {
    pub fn new(path: Option<PathBuf>) -> Result<Self, CrabError> {
        if let Some(p) = path {
            if p.exists() {
                 Ok(InputSource::File(p))
            } else {
                 Err(CrabError::Cli(format!("File not found: {:?}", p)))
            }
        } else {
            // Read from stdin
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            
            // We'll read up to MAX_INMEM_PDF_BYTES + 1 to see if we go over
            let mut buffer = Vec::with_capacity(1024 * 1024); // Start with 1MB capacity
            
            // Strategy: Read into buffer. If it gets too large, switch to temp file.
            // But `read_to_end` is easiest for small files.
            
            // Let's implement a loop that reads chunks.
            let mut total_read = 0;
            let mut chunk = [0u8; 8192];
            
            loop {
                let n = handle.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                total_read += n;
                buffer.extend_from_slice(&chunk[..n]);
                
                if total_read > MAX_INMEM_PDF_BYTES {
                    // Switch to temp file
                    let mut temp_file = NamedTempFile::new()?;
                    temp_file.write_all(&buffer)?;
                    // Continue reading remainder from stdin to temp_file
                    io::copy(&mut handle, &mut temp_file)?;
                    return Ok(InputSource::TempFile(temp_file));
                }
            }

            Ok(InputSource::StdinBytes(buffer))
        }
    }
}
