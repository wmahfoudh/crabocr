#include "wrapper.h"
#include <string.h>

// No-op warning callback to silence MuPDF warnings
void my_warning_cb(void *user, const char *message) {
  (void)user;
  (void)message;
}

fz_context *my_new_context() {
  fz_context *ctx = fz_new_context(NULL, NULL, FZ_STORE_DEFAULT);
  if (ctx) {
    fz_set_warning_callback(ctx, my_warning_cb, NULL);
  }
  return ctx;
}

void my_drop_context(fz_context *ctx) {
  if (ctx)
    fz_drop_context(ctx);
}

int my_open_document(fz_context *ctx, const char *filename,
                     fz_document **doc_out, char *err_out, size_t err_len) {
  if (!ctx || !filename || !doc_out)
    return -1;
  *doc_out = NULL;

  fz_try(ctx) {
    fz_register_document_handlers(ctx);
    *doc_out = fz_open_document(ctx, filename);
  }
  fz_catch(ctx) {
    if (err_out)
      strncpy(err_out, fz_caught_message(ctx), err_len - 1);
    return 1;
  }
  return 0;
}

void my_drop_document(fz_context *ctx, fz_document *doc) {
  if (ctx && doc)
    fz_drop_document(ctx, doc);
}

int my_count_pages(fz_context *ctx, fz_document *doc, int *count_out,
                   char *err_out, size_t err_len) {
  if (!ctx || !doc || !count_out)
    return -1;

  fz_try(ctx) { *count_out = fz_count_pages(ctx, doc); }
  fz_catch(ctx) {
    if (err_out)
      strncpy(err_out, fz_caught_message(ctx), err_len - 1);
    return 1;
  }
  return 0;
}

int my_render_page(fz_context *ctx, fz_document *doc, int page_number, int dpi,
                   fz_pixmap **pix_out, char *err_out, size_t err_len) {
  if (!ctx || !doc || !pix_out)
    return -1;

  fz_try(ctx) {
    // Load page
    fz_page *page = fz_load_page(ctx, doc, page_number);

    // Calculate matrix
    // Default dpi is 72. Scale = dpi / 72.
    float scale = (float)dpi / 72.0f;
    fz_matrix ctm = fz_scale(scale, scale);

    // Render
    // Use RGB (no alpha if possible, but fz_device_rgb returns alpha?)
    // fz_device_rgb(ctx) returns a colorspace.
    // We want RGB.
    *pix_out = fz_new_pixmap_from_page(ctx, page, ctm, fz_device_rgb(ctx), 0);

    fz_drop_page(ctx, page);
  }
  fz_catch(ctx) {
    if (err_out)
      strncpy(err_out, fz_caught_message(ctx), err_len - 1);
    return 1;
  }
  return 0;
}

void my_drop_pixmap(fz_context *ctx, fz_pixmap *pix) {
  if (ctx && pix)
    fz_drop_pixmap(ctx, pix);
}

unsigned char *my_pixmap_samples(fz_context *ctx, fz_pixmap *pix) {
  return fz_pixmap_samples(ctx, pix);
}

int my_pixmap_width(fz_context *ctx, fz_pixmap *pix) {
  return fz_pixmap_width(ctx, pix);
}
int my_pixmap_height(fz_context *ctx, fz_pixmap *pix) {
  return fz_pixmap_height(ctx, pix);
}
int my_pixmap_stride(fz_context *ctx, fz_pixmap *pix) {
  return fz_pixmap_stride(ctx, pix);
}
int my_pixmap_n(fz_context *ctx, fz_pixmap *pix) {
  (void)ctx;
  return pix->n;
}
