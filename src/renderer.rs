use crate::errors::CrabError;
use std::ffi::CString;
use std::path::Path;
use std::ptr;

// Include generated bindings
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod sys {
    include!(concat!(env!("OUT_DIR"), "/bindings_mupdf.rs"));
}
use sys::*;

pub struct Renderer {
    ctx: *mut fz_context,
}

pub struct Document {
    doc: *mut fz_document,
}

impl Renderer {
    pub fn new() -> Result<Self, CrabError> {
        unsafe {
            let ctx = my_new_context();
            if ctx.is_null() {
                return Err(CrabError::Internal("Failed to create MuPDF context".into()));
            }
            Ok(Self { ctx })
        }
    }

    pub fn open(&self, path: &Path) -> Result<Document, CrabError> {
        let path_str = path.to_str().ok_or_else(|| CrabError::Input(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path encoding")))?;
        let c_path = CString::new(path_str).map_err(|_| CrabError::Input(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Null byte in path")))?;

        unsafe {
            let mut doc: *mut fz_document = ptr::null_mut();
            let mut err_buf = [0i8; 256];
            let ret = my_open_document(self.ctx, c_path.as_ptr(), &mut doc, err_buf.as_mut_ptr(), err_buf.len());
            
            if ret != 0 {
                let err_msg = std::ffi::CStr::from_ptr(err_buf.as_ptr()).to_string_lossy().into_owned();
                return Err(CrabError::Pdf(format!("Failed to open document: {}", err_msg)));
            }
            
            Ok(Document { doc })
        }
    }
    
    pub fn page_count(&self, doc: &Document) -> Result<i32, CrabError> {
        unsafe {
            let mut count = 0;
            let mut err_buf = [0i8; 256];
            let ret = my_count_pages(self.ctx, doc.doc, &mut count, err_buf.as_mut_ptr(), err_buf.len());
            
            if ret != 0 {
                let err_msg = std::ffi::CStr::from_ptr(err_buf.as_ptr()).to_string_lossy().into_owned();
                return Err(CrabError::Pdf(format!("Failed to count pages: {}", err_msg)));
            }
            Ok(count)
        }
    }

    pub fn render_page(&self, doc: &Document, page_number: i32, dpi: i32) -> Result<Pixmap, CrabError> {
        unsafe {
            let mut pix: *mut fz_pixmap = ptr::null_mut();
            let mut err_buf = [0i8; 256];
            let ret = my_render_page(self.ctx, doc.doc, page_number, dpi, &mut pix, err_buf.as_mut_ptr(), err_buf.len());

            if ret != 0 {
                let err_msg = std::ffi::CStr::from_ptr(err_buf.as_ptr()).to_string_lossy().into_owned();
                return Err(CrabError::Pdf(format!("Failed to render page {}: {}", page_number, err_msg)));
            }

            Ok(Pixmap { pix })
        }
    }
    
    /// Extract XFA XML data from the document if present.
    /// Returns None if no XFA data exists.
    pub fn extract_xfa(&self, doc: &Document) -> Option<String> {
        unsafe {
            let mut len: usize = 0;
            let mut err_buf = [0i8; 256];
            
            let xfa_ptr = my_extract_xfa(
                self.ctx,
                doc.doc,
                &mut len,
                err_buf.as_mut_ptr(),
                err_buf.len(),
            );
            
            if xfa_ptr.is_null() || len == 0 {
                return None;
            }
            
            // Copy to Rust String before freeing C memory
            let slice = std::slice::from_raw_parts(xfa_ptr as *const u8, len);
            let result = String::from_utf8_lossy(slice).into_owned();
            
            // Free the C-allocated memory
            my_free_xfa(self.ctx, xfa_ptr);
            
            Some(result)
        }
    }

    /// Extract structured text from a page.
    pub fn extract_text(&self, doc: &Document, page_number: i32) -> Result<String, CrabError> {
        unsafe {
            let mut err_buf = [0i8; 256];
            let text_ptr = my_extract_text(
                self.ctx,
                doc.doc,
                page_number,
                err_buf.as_mut_ptr(),
                err_buf.len(),
            );

            if text_ptr.is_null() {
                 let err_msg = std::ffi::CStr::from_ptr(err_buf.as_ptr()).to_string_lossy().into_owned();
                 return Err(CrabError::Pdf(format!("Failed to extract text from page {}: {}", page_number, err_msg)));
            }

            let c_str = std::ffi::CStr::from_ptr(text_ptr);
            let text = c_str.to_string_lossy().into_owned();
            
            my_free_text(self.ctx, text_ptr);
            
            Ok(text)
        }
    }

}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            my_drop_context(self.ctx);
        }
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        // Document cleanup is handled manually via `drop_with` because it requires the context.
    }
}

pub struct Pixmap {
    pix: *mut fz_pixmap,
}

/// A wrapper around a MuPDF pixmap.
///
/// # Safety
///
/// Like `Document`, this struct does not automatically free its C resources on drop
/// because it needs the context. Use `drop_with` to prevent leaks.
///
/// The `samples` method returns a slice backed by C memory. This slice is valid
/// as long as the `Pixmap` is not dropped/freed.
/// A wrapper around a MuPDF pixmap.
///
/// # Safety
///
/// Like `Document`, this struct does not automatically free its C resources on drop
/// because it needs the context. Use `drop_with` to prevent leaks.
///
/// The `samples` method returns a slice backed by C memory. This slice is valid
/// as long as the `Pixmap` is not dropped/freed.
impl Pixmap {
    pub fn width(&self, ctx: &Renderer) -> i32 {
        unsafe { my_pixmap_width(ctx.ctx, self.pix) }
    }
    pub fn height(&self, ctx: &Renderer) -> i32 {
        unsafe { my_pixmap_height(ctx.ctx, self.pix) }
    }
    pub fn stride(&self, ctx: &Renderer) -> i32 {
        unsafe { my_pixmap_stride(ctx.ctx, self.pix) }
    }
    pub fn n(&self, ctx: &Renderer) -> i32 {
        unsafe { my_pixmap_n(ctx.ctx, self.pix) }
    }
    pub fn samples(&self, ctx: &Renderer) -> &[u8] {
        unsafe {
            let ptr = my_pixmap_samples(ctx.ctx, self.pix);
            let len = (self.stride(ctx) * self.height(ctx)) as usize;
            std::slice::from_raw_parts(ptr, len)
        }
    }
    
    pub fn drop_with(&mut self, ctx: &Renderer) {
        unsafe {
             if !self.pix.is_null() {
                 my_drop_pixmap(ctx.ctx, self.pix);
                 self.pix = ptr::null_mut();
             }
        }
    }
}

// Manually dropping document
impl Document {
    pub fn drop_with(&mut self, ctx: &Renderer) {
        unsafe {
             if !self.doc.is_null() {
                 my_drop_document(ctx.ctx, self.doc);
                 self.doc = ptr::null_mut();
             }
        }
    }
}

