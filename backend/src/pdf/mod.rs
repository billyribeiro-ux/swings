//! EC-06: PDF generation.
//!
//! Every function in this module is pure — it accepts the data it needs as
//! owned / borrowed references and returns the PDF bytes. I/O (writing to
//! disk, R2, or the HTTP response) is the handler layer's problem.
//!
//! We use [`printpdf`] 0.7 with the Helvetica built-in font. Built-ins
//! avoid shipping a TTF for first-line invoices and keep the binary
//! footprint small; a follow-up can swap in a web-safe TTF when we start
//! embedding non-Latin characters.

pub mod invoice;

pub use invoice::{generate_invoice, generate_receipt, InvoiceData, InvoiceItem};
