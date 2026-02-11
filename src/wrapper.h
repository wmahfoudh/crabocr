#include "mupdf/fitz.h"
#include "mupdf/pdf.h"

typedef struct {
  fz_context *ctx;
  fz_document *doc;
} PdfDocument;

typedef struct {
  int width;
  int height;
  int stride;
  int n; // components
  unsigned char *samples;
  // ownership managed by fz_pixmap, this struct is just for checking
} RenderResult;

// Returns NULL on error (message printed to stderr by mupdf default, or we can
// capture)
fz_context *my_new_context();
void my_drop_context(fz_context *ctx);

// Returns non-zero on error using error buffer
int my_open_document(fz_context *ctx, const char *filename,
                     fz_document **doc_out, char *err_out, size_t err_len);
void my_drop_document(fz_context *ctx, fz_document *doc);

int my_count_pages(fz_context *ctx, fz_document *doc, int *count_out,
                   char *err_out, size_t err_len);

// Returns pixmap or error
int my_render_page(fz_context *ctx, fz_document *doc, int page_number, int dpi,
                   fz_pixmap **pix_out, char *err_out, size_t err_len);

void my_drop_pixmap(fz_context *ctx, fz_pixmap *pix);

// Accessors for pixmap
unsigned char *my_pixmap_samples(fz_context *ctx, fz_pixmap *pix);
int my_pixmap_width(fz_context *ctx, fz_pixmap *pix);
int my_pixmap_height(fz_context *ctx, fz_pixmap *pix);
int my_pixmap_stride(fz_context *ctx, fz_pixmap *pix);
int my_pixmap_n(fz_context *ctx, fz_pixmap *pix);

// XFA extraction
// Returns dynamically allocated UTF-8 string, or NULL if no XFA data.
// Caller must free with my_free_xfa(). len_out receives string length.
char *my_extract_xfa(fz_context *ctx, fz_document *doc, size_t *len_out,
                     char *err_out, size_t err_len);
void my_free_xfa(fz_context *ctx, char *xfa_data);

// Text extraction
// Returns dynamically allocated UTF-8 string, or NULL if no text.
// Caller must free with my_free_text().
char *my_extract_text(fz_context *ctx, fz_document *doc, int page_number,
                      char *err_out, size_t err_len);
void my_free_text(fz_context *ctx, char *text);
