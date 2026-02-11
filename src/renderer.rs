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
    // Context borrowing logic:
    // In C, doc depends on ctx. In Rust, we need to ensure ctx lives longer.
    // Simplifying: internal pointer, careful usage.
    // Or we can make Document borrow Renderer?
    // 'a lifetime.
}

// But wait, fz_document doesn't store ctx usually, operations need ctx passed.
// So methods on Document need &Renderer (or &mut Renderer).

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
    
    // Safety check helper
    
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
        // In this design, Document doesn't own context, so it can't drop itself using context?
        // Wait, fz_drop_document needs ctx.
        // If Document drops without context, leak?
        // This is a common FFI problem.
        // Solution: wrappers usually hold reference to Context or Context is global?
        // Or Document drop is no-op if we don't have ctx, but we need to drop it manually?
        // Better: `Renderer` should drop `Document`.
        // Or `Document` holds `Rc<Context>` equivalent?
        // For simplicity: `Document` just holds the pointer, and we add `close_document` to `Renderer`.
        // Or `unsafe impl Send` and pass `&Renderer`.
        
        // Actually, `fz_drop_document` takes `ctx`.
        // If `Document` is dropped, we can't easily get `ctx` unless we store it.
        // But `ctx` pointer is `*mut fz_context`.
        // If `Renderer` is dropped first, `ctx` is invalid.
        // So `Document` must not outlive `Renderer`.
        // Lifetimes can enforce this.
        // But `Drop` trait cannot take arguments.
        // So we store `ctx` in `Document` too?
        // Yes, as long as we ensure `Document` lifetime < `Renderer` lifetime.
        // But raw pointers don't track ownership.
        
        // We will store `ctx` in `Document` purely for dropping purposes, 
        // AND use phantom lifetime to tie it to `Renderer`.
        // Or just let `Renderer` have `close(doc)` and `Document` having a `dropped` flag?
        // The safest rust way: `Document<'a>` holds `&'a Renderer`.
        // But `Drop` implementation cannot have lifetime parameters if struct is generic?
        // Actually struct can have lifetime.
        // But if `Renderer` moves, the reference breaks? `Renderer` shouldn't move if pinned.
        
        // Alternative: shared ownership of context via `Rc` or `Arc`?
        // `Renderer` owns context. `Document` holds `Arc<ContextWrapper>`?
        // Too complex for now.
        
        // Pragmatic approach:
        // `Document` does nothing in Drop? Then we leak.
        // `Document` stores `ctx`.
        // We assume `Renderer` outlives `Document`.
    }
}

pub struct Pixmap {
    pix: *mut fz_pixmap,
    // Needs context to drop? Yes.
    // Store ctx here too?
}

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

