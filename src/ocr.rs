use crate::errors::CrabError;
use std::ffi::{CStr, CString};
use crate::renderer::Renderer;

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(clippy::all)]
#[allow(warnings)]
mod sys {
    include!(concat!(env!("OUT_DIR"), "/bindings_tesseract.rs"));

    // Manual definitions for functions safely assumed to be in libtesseract
    // that might be missing from generated bindings.
    
    extern "C" {
        pub fn setMsgSeverity(severity: i32);
    }
}
use sys::*;

// Helper for silencing stderr.
//
// Warning: This struct modifies the global file descriptor table (stderr).
// It is NOT thread-safe. Using this in a multi-threaded environment where
// other threads write to stderr may result in lost logs.
struct StderrSilencer {
    original_stderr: i32,
}

impl StderrSilencer {
    fn new(null_fd: i32) -> Option<Self> {
        let stderr_fd = 2;
        unsafe {
            let original = libc::dup(stderr_fd);
            if original == -1 {
                return None;
            }
            
            // Redirect stderr to /dev/null
            if libc::dup2(null_fd, stderr_fd) == -1 {
                libc::close(original);
                return None;
            }
            
            Some(Self {
                original_stderr: original,
            })
        }
    }
}

impl Drop for StderrSilencer {
    fn drop(&mut self) {
        let stderr_fd = 2;
        unsafe {
            // Restore stderr
            libc::dup2(self.original_stderr, stderr_fd);
            libc::close(self.original_stderr);
        }
    }
}

pub struct Ocr {
    handle: *mut TessBaseAPI,
    // Keep file open to reuse FD
    _dev_null: std::fs::File,
}

impl Ocr {
    pub fn new(lang: &str) -> Result<Self, CrabError> {
        use std::os::fd::AsRawFd;
        
        let dev_null = std::fs::File::open("/dev/null")
            .map_err(|e| CrabError::Internal(format!("Failed to open /dev/null: {}", e)))?;
        let null_fd = dev_null.as_raw_fd();
        
        // Silence entire initialization to catch Leptonica errors
        let _silencer = StderrSilencer::new(null_fd);
        
        unsafe {
            let handle = TessBaseAPICreate();
            if handle.is_null() {
                return Err(CrabError::Ocr("Failed to create Tesseract handle".into()));
            }

            // Helper closure to set Tesseract variables.
            let set_var = |name: &str, val: &str| {
                let c_name = CString::new(name).unwrap();
                let c_val = CString::new(val).unwrap();
                TessBaseAPISetVariable(handle, c_name.as_ptr(), c_val.as_ptr());
            };

            // 1. Dictionary Support: "1" to enable
            set_var("tessedit_enable_doc_dict", "1");

            // 2. Layout Preservation: "0" to fix random paragraph splitting
            set_var("preserve_interword_spaces", "0");
            
            // Resolve datapath
            let possible_paths = vec![
                std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("tessdata"))), 
                Some(std::path::PathBuf::from("tessdata")), 
            ];
            
            for p in possible_paths.into_iter().flatten() {
                 if p.exists() && p.is_dir() {
                     if let Ok(abs_path) = std::fs::canonicalize(&p) {
                         std::env::set_var("TESSDATA_PREFIX", abs_path);
                         break;
                     }
                 }
            }
            
            // Set message severity
            setMsgSeverity(6); // L_SEVERITY_NONE
            
            let ptr_datapath = std::ptr::null(); // Use env var
            let c_lang = CString::new(lang).map_err(|_| CrabError::Input(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid lang string")))?;

            // 3. Engine Mode: LSTM_ONLY (1)
            let ret = TessBaseAPIInit2(handle, ptr_datapath, c_lang.as_ptr(), TessOcrEngineMode_OEM_LSTM_ONLY);
            
            if ret != 0 {
                // If we return, silencer drops and restores stderr.
                TessBaseAPIDelete(handle);
                // Drop silencer to allow error printing
                drop(_silencer);
                return Err(CrabError::Ocr(format!("Failed to initialize Tesseract with lang '{}' (OEM=LSTM_ONLY)", lang)));
            }
            
            // Check if 'osd.traineddata' is available in TESSDATA_PREFIX.
            let psm = if let Ok(prefix) = std::env::var("TESSDATA_PREFIX") {
                let osd_path = std::path::Path::new(&prefix).join("osd.traineddata");
                if osd_path.exists() {
                     TessPageSegMode_PSM_AUTO_OSD
                } else {
                     // Print warning to stdout as stderr might be silenced.
                     println!("Warning: 'osd.traineddata' not found in {:?}. Auto-rotation (OSD) disabled. Falling back to PSM_AUTO.", prefix);
                     TessPageSegMode_PSM_AUTO
                }
            } else {
                 TessPageSegMode_PSM_AUTO
            };
            
            TessBaseAPISetPageSegMode(handle, psm);
            
            // StderrSilencer is dropped here, restoring stderr.
            Ok(Self { 
                handle, 
                _dev_null: dev_null 
            })
        }
    }
    
    pub fn recognize(&self, pix: &crate::renderer::Pixmap, renderer: &Renderer, dpi: i32) -> Result<String, CrabError> {
        use std::os::fd::AsRawFd;
        // Silence entire recognition to catch OSD warnings
        let _silencer = StderrSilencer::new(self._dev_null.as_raw_fd());
        
        unsafe {
            // Silence everything in recognize to catch 'pixReadMemTiff' from SetImage or Recognize.
            
            let width = pix.width(renderer);
            let height = pix.height(renderer);
            let stride = pix.stride(renderer);
            let channels = pix.n(renderer); 
            let samples = pix.samples(renderer);

            // 2. Image Integrity
            TessBaseAPISetImage(self.handle, samples.as_ptr(), width, height, channels, stride);

            // 1. Active DPI (Must be called AFTER SetImage)
            TessBaseAPISetSourceResolution(self.handle, dpi);
            
            // Recognize
            if TessBaseAPIRecognize(self.handle, std::ptr::null_mut()) != 0 {
                 return Err(CrabError::Ocr("Error during recognition".into()));
            }

            // Check confidence score.
            // Reject output if the mean confidence is below 60 (out of 100).
            // This filters out noise from empty or garbled pages.
            let mean_conf = TessBaseAPIMeanTextConf(self.handle);
            if mean_conf < 60 {
                TessBaseAPIClear(self.handle);
                return Ok(String::new());
            }

            let text_ptr = TessBaseAPIGetUTF8Text(self.handle);
            if text_ptr.is_null() {
                return Ok(String::new()); 
            }
            
            let text = CStr::from_ptr(text_ptr).to_string_lossy().into_owned();
            TessDeleteText(text_ptr);
            TessBaseAPIClear(self.handle);
            
            Ok(text)
        }
    }
}

impl Drop for Ocr {
    fn drop(&mut self) {
        unsafe {
            TessBaseAPIEnd(self.handle);
            TessBaseAPIDelete(self.handle);
        }
    }
}
