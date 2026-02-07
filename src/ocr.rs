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

    extern "C" {
        pub fn setMsgSeverity(severity: i32);
    }
}
use sys::*;

pub struct Ocr {
    handle: *mut TessBaseAPI,
}

impl Ocr {
    pub fn new(lang: &str) -> Result<Self, CrabError> {
        unsafe {
            let handle = TessBaseAPICreate();
            if handle.is_null() {
                return Err(CrabError::Ocr("Failed to create Tesseract handle".into()));
            }
            
            // Resolve datapath. Tesseract needs the PARENT directory of 'tessdata'.
            // If binary is in /app/bin, and tessdata in /app/tessdata, we pass /app/.
            // If binary is in target/debug/deps, and tessdata in . (repo root), we pass .
            // We'll check various locations.
            
            let possible_paths = vec![
                std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("tessdata"))), // Next to exe
                Some(std::path::PathBuf::from("tessdata")), // Current dir
            ];
            
            for p in possible_paths.into_iter().flatten() {
                 if p.exists() && p.is_dir() {
                     // Found tessdata dir. 
                     // Tesseract expects TESSDATA_PREFIX to point TO the directory containing .traineddata files.
                     // IMPORTANT: The C-API Init() takes 'datapath' which is PARENT of 'tessdata/'.
                     // BUT setting env var TESSDATA_PREFIX usually points DIRECTLY to 'tessdata/'.
                     // Let's rely on TESSDATA_PREFIX env var for robustness.
                     
                     if let Ok(abs_path) = std::fs::canonicalize(&p) {
                         std::env::set_var("TESSDATA_PREFIX", abs_path);
                         break;
                     }
                 }
            }
            

            // Set message severity to avoid spamming stderr about missinglibs
            // L_SEVERITY_NONE = 6 (usually). Bindings might define it.
            // Let's assume bindings exist or use literal if constant is missing.
            // Check if setMsgSeverity is bound? Bindgen usually binds all public functions in headers included.
            // If not, we can declare externblock or try using it.
            // Actually, we can just compile and see. If failing, we'll fix.
            // To be safe, we can try to call it.
            
            // The function signature: void setMsgSeverity(l_int32 severity);
            // L_SEVERITY_NONE is defined in environ.h as 6.
            
            // We need to define L_SEVERITY_NONE manually if not in bindings.
            // Let's try to use the binding's L_SEVERITY_NONE if available.
            
            setMsgSeverity(6); // 6 = L_SEVERITY_NONE
            
            // If we set env var, we can pass NULL to Init.
            let ptr_datapath = std::ptr::null();
            
            let c_lang = CString::new(lang).map_err(|_| CrabError::Input(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid lang string")))?;

            let ret = TessBaseAPIInit3(handle, ptr_datapath, c_lang.as_ptr());
            
            if ret != 0 {
                TessBaseAPIDelete(handle);
                return Err(CrabError::Ocr(format!("Failed to initialize Tesseract with lang '{}' (TESSDATA_PREFIX set)", lang)));
            }
            
            // Set page segmentation mode? Default is usually correct (PSM_AUTO).
            
            Ok(Self { handle })
        }
    }
    
    pub fn recognize(&self, pix: &crate::renderer::Pixmap, renderer: &Renderer) -> Result<String, CrabError> {
        unsafe {
            let width = pix.width(renderer);
            let height = pix.height(renderer);
            let stride = pix.stride(renderer);
            let channels = pix.n(renderer); 
            let samples = pix.samples(renderer);
            
            // Bytes per pixel. MuPDF 0=Gray, 3=RGB usually.
            // Check assumption.
            // Tesseract SetImage takes bytes_per_pixel.
            
            TessBaseAPISetImage(self.handle, samples.as_ptr(), width, height, channels, stride);
            
            // Recognize
            if TessBaseAPIRecognize(self.handle, std::ptr::null_mut()) != 0 {
                 return Err(CrabError::Ocr("Error during recognition".into()));
            }

            let text_ptr = TessBaseAPIGetUTF8Text(self.handle);
            if text_ptr.is_null() {
                return Ok(String::new()); 
            }
            
            let text = CStr::from_ptr(text_ptr).to_string_lossy().into_owned();
            TessDeleteText(text_ptr);
            TessBaseAPIClear(self.handle); // Clear image and result
            
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
