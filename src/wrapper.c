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

char *my_extract_xfa(fz_context *ctx, fz_document *doc, size_t *len_out,
                     char *err_out, size_t err_len) {
  if (!ctx || !doc || !len_out)
    return NULL;

  *len_out = 0;
  char *volatile result = NULL;

  fz_try(ctx) {
    // Check if this is a PDF document
    pdf_document *pdoc = pdf_specifics(ctx, doc);
    if (!pdoc) {
      // Not a PDF, no XFA possible
      return NULL;
    }

    // Navigate: trailer -> Root -> AcroForm -> XFA
    pdf_obj *trailer = pdf_trailer(ctx, pdoc);
    if (!trailer)
      break;

    pdf_obj *root = pdf_dict_gets(ctx, trailer, "Root");
    if (!root)
      break;

    pdf_obj *acroform = pdf_dict_gets(ctx, root, "AcroForm");
    if (!acroform)
      break;

    pdf_obj *xfa = pdf_dict_gets(ctx, acroform, "XFA");
    if (!xfa)
      break;

    // XFA can be either a stream or an array of [name, stream] pairs
    fz_buffer *combined = fz_new_buffer(ctx, 1024);

    if (pdf_is_stream(ctx, xfa)) {
      // Single stream
      fz_buffer *buf = pdf_load_stream(ctx, xfa);
      fz_append_buffer(ctx, combined, buf);
      fz_drop_buffer(ctx, buf);
    } else if (pdf_is_array(ctx, xfa)) {
      // Array of [name, stream, name, stream, ...]
      int len = pdf_array_len(ctx, xfa);
      for (int i = 1; i < len;
           i += 2) { // Start at 1 to get streams (0 is name)
        pdf_obj *stream_obj = pdf_array_get(ctx, xfa, i);
        if (pdf_is_stream(ctx, stream_obj)) {
          fz_buffer *buf = pdf_load_stream(ctx, stream_obj);
          fz_append_buffer(ctx, combined, buf);
          fz_drop_buffer(ctx, buf);
        }
      }
    }

    // Extract data from buffer
    unsigned char *data = NULL;
    size_t data_len = fz_buffer_extract(ctx, combined, &data);
    fz_drop_buffer(ctx, combined);

    if (data_len > 0 && data != NULL) {
      // Ensure null termination for string
      result = fz_malloc(ctx, data_len + 1);
      memcpy(result, data, data_len);
      result[data_len] = '\0';
      *len_out = data_len;
    }
    fz_free(ctx, data);
  }
  fz_catch(ctx) {
    if (err_out)
      strncpy(err_out, fz_caught_message(ctx), err_len - 1);
    return NULL;
  }

  return result;
}

void my_free_xfa(fz_context *ctx, char *xfa_data) {
  if (ctx && xfa_data)
    fz_free(ctx, xfa_data);
}

char *my_extract_text(fz_context *ctx, fz_document *doc, int page_number,
                      char *err_out, size_t err_len) {
  if (!ctx || !doc)
    return NULL;

  char *volatile result = NULL;

  fz_try(ctx) {
    fz_page *page = fz_load_page(ctx, doc, page_number);

    // Create a structured text page
    fz_stext_page *text_page = fz_new_stext_page(ctx, fz_bound_page(ctx, page));

    // Parse the page for text
    fz_stext_options opts;
    memset(&opts, 0, sizeof(opts));
    // opts.flags = FZ_STEXT_PRESERVE_IMAGES; // If we wanted images, but we
    // want text.

    // Create a device to extract text
    fz_device *dev = fz_new_stext_device(ctx, text_page, &opts);

    // Run the page through the device
    fz_run_page(ctx, page, dev, fz_identity, NULL);

    fz_close_device(ctx, dev);
    fz_drop_device(ctx, dev);

    // Extract text from the text page to a buffer
    // fz_print_stext_page_as_text outputs to an output stream.
    // We want it in a buffer.

    fz_buffer *buf = fz_new_buffer(ctx, 1024);
    fz_output *out = fz_new_output_with_buffer(ctx, buf);

    fz_print_stext_page_as_text(ctx, out, text_page);

    fz_close_output(ctx, out);
    fz_drop_output(ctx, out);
    fz_drop_stext_page(ctx, text_page);
    fz_drop_page(ctx, page);

    // Get string from buffer
    unsigned char *data = NULL;
    size_t len = fz_buffer_extract(ctx, buf, &data);
    fz_drop_buffer(ctx, buf);

    if (len > 0 && data != NULL) {
      // Ensure null termination
      result = fz_malloc(ctx, len + 1);
      memcpy(result, data, len);
      result[len] = '\0';
    } else {
      // Empty string if no text found, to differentiate from error (NULL)
      result = fz_malloc(ctx, 1);
      result[0] = '\0';
    }
    fz_free(ctx, data);
  }
  fz_catch(ctx) {
    if (err_out)
      strncpy(err_out, fz_caught_message(ctx), err_len - 1);
    return NULL;
  }

  return result;
}

void my_free_text(fz_context *ctx, char *text) {
  if (ctx && text)
    fz_free(ctx, text);
}
