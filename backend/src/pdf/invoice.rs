//! EC-06: Invoice + receipt PDF generator.
//!
//! US Letter (215.9 mm × 279.4 mm), Helvetica built-in font. The layout is:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │ INVOICE                    Order: ORD-2026-0042 │
//! │                              Date: 2026-04-17   │
//! │                                                 │
//! │ Bill to:                                        │
//! │ <email>                                         │
//! │                                                 │
//! │ ─────────────────────────────────────────────── │
//! │ QTY  ITEM                  UNIT      LINE       │
//! │ 1    Example product       $10.00    $10.00     │
//! │ 2    Another product       $ 5.00    $10.00     │
//! │ ─────────────────────────────────────────────── │
//! │                       Subtotal:      $20.00     │
//! │                       Discount:     -$ 2.00     │
//! │                            Tax:      $ 1.80     │
//! │                          Total:      $19.80     │
//! │                                                 │
//! │ Thank you for your business.                    │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! The generator accepts an [`InvoiceData`] trait-object so we don't need
//! to hard-depend on any concrete `Order` / `OrderItem` shape. Handlers
//! that already carry `crate::commerce::orders::{Order, OrderItem}` can
//! implement the trait in a single `impl` block (see the bottom of this
//! file for the concrete bridge).

use chrono::{DateTime, Utc};
use printpdf::{BuiltinFont, Mm, PdfDocument};

use crate::commerce::orders::{Order, OrderItem};

/// Everything the PDF generator needs from an order to lay out an invoice
/// or a receipt.
///
/// Currencies are minor units (`i64` cents). The generator formats them
/// with two decimals regardless of currency — at the time of writing we
/// only ship USD / EUR / GBP, all of which use 2 fraction digits. A
/// follow-up can key off the currency code when we add JPY (0 fractions)
/// or KWD (3 fractions).
pub trait InvoiceData {
    fn number(&self) -> &str;
    fn created_at(&self) -> DateTime<Utc>;
    fn email(&self) -> &str;
    fn currency(&self) -> &str;
    fn subtotal_cents(&self) -> i64;
    fn discount_cents(&self) -> i64;
    fn tax_cents(&self) -> i64;
    fn total_cents(&self) -> i64;
    fn items(&self) -> Box<dyn Iterator<Item = &dyn InvoiceItem> + '_>;
}

/// Everything the PDF generator needs from a single line item.
pub trait InvoiceItem {
    fn name(&self) -> &str;
    fn quantity(&self) -> i32;
    fn unit_price_cents(&self) -> i64;
    fn line_total_cents(&self) -> i64;
}

// ── Concrete blanket for the real Order type ──────────────────────────

impl InvoiceItem for OrderItem {
    fn name(&self) -> &str {
        &self.name
    }
    fn quantity(&self) -> i32 {
        self.quantity
    }
    fn unit_price_cents(&self) -> i64 {
        self.unit_price_cents
    }
    fn line_total_cents(&self) -> i64 {
        self.line_total_cents
    }
}

/// Bridge adapter so handler code can pass its already-loaded order +
/// items through without implementing the trait by hand. The adapter owns
/// borrowed references to the underlying row so no copy is made.
pub struct OrderInvoice<'a> {
    pub order: &'a Order,
    pub items: &'a [OrderItem],
}

impl<'a> InvoiceData for OrderInvoice<'a> {
    fn number(&self) -> &str {
        &self.order.number
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.order.created_at
    }
    fn email(&self) -> &str {
        &self.order.email
    }
    fn currency(&self) -> &str {
        &self.order.currency
    }
    fn subtotal_cents(&self) -> i64 {
        self.order.subtotal_cents
    }
    fn discount_cents(&self) -> i64 {
        self.order.discount_cents
    }
    fn tax_cents(&self) -> i64 {
        self.order.tax_cents
    }
    fn total_cents(&self) -> i64 {
        self.order.total_cents
    }
    fn items(&self) -> Box<dyn Iterator<Item = &dyn InvoiceItem> + '_> {
        Box::new(self.items.iter().map(|i| i as &dyn InvoiceItem))
    }
}

// ── Layout constants (mm) ──────────────────────────────────────────────

// US Letter: 8.5 × 11 inches → 215.9 × 279.4 mm.
const PAGE_W_MM: f32 = 215.9;
const PAGE_H_MM: f32 = 279.4;
const MARGIN_MM: f32 = 20.0;
/// Starting y-coordinate (mm from bottom) for the header row.
const HEADER_Y_MM: f32 = 260.0;
/// Row height in the item table.
const ROW_H_MM: f32 = 6.0;

// ── Public API ─────────────────────────────────────────────────────────

/// Render an invoice PDF. Uses "INVOICE" as the top-of-page heading.
#[must_use]
pub fn generate_invoice(data: &dyn InvoiceData) -> Vec<u8> {
    render(data, "INVOICE", "Thank you for your business.")
}

/// Render a receipt PDF. Identical layout to [`generate_invoice`], with a
/// "RECEIPT" heading and a payment-acknowledged footer copy.
#[must_use]
pub fn generate_receipt(data: &dyn InvoiceData) -> Vec<u8> {
    render(data, "RECEIPT", "Payment received — thank you.")
}

/// Internal renderer. Returns a valid, non-trivial PDF even on degenerate
/// inputs (empty items, zero totals); the fallback path kicks in only on
/// the theoretically-impossible printpdf serialization failure, where we
/// emit a minimal but valid `%PDF-1.3` document so callers never have to
/// reason about a broken body.
fn render(data: &dyn InvoiceData, heading: &str, footer: &str) -> Vec<u8> {
    let (doc, page1, layer1) = PdfDocument::new(
        format!("Invoice {}", data.number()),
        Mm(PAGE_W_MM),
        Mm(PAGE_H_MM),
        "Layer 1",
    );
    let layer = doc.get_page(page1).get_layer(layer1);

    // Fonts — Helvetica is always available in the PDF spec so unwrap_or
    // here reduces to a no-realistic-failure path. We still handle the
    // error defensively by falling back to the empty-pdf bytes.
    let font_regular = match doc.add_builtin_font(BuiltinFont::Helvetica) {
        Ok(f) => f,
        Err(_) => return fallback_pdf(),
    };
    let font_bold = match doc.add_builtin_font(BuiltinFont::HelveticaBold) {
        Ok(f) => f,
        Err(_) => return fallback_pdf(),
    };

    // ── Header ────────────────────────────────────────────────────────
    layer.use_text(heading, 24.0, Mm(MARGIN_MM), Mm(HEADER_Y_MM), &font_bold);

    let right_x = PAGE_W_MM - MARGIN_MM - 70.0;
    layer.use_text(
        format!("Order: {}", data.number()),
        11.0,
        Mm(right_x),
        Mm(HEADER_Y_MM + 6.0),
        &font_regular,
    );
    layer.use_text(
        format!("Date: {}", data.created_at().format("%Y-%m-%d")),
        11.0,
        Mm(right_x),
        Mm(HEADER_Y_MM),
        &font_regular,
    );

    // ── Bill to ───────────────────────────────────────────────────────
    let mut y = HEADER_Y_MM - 20.0;
    layer.use_text("Bill to:", 11.0, Mm(MARGIN_MM), Mm(y), &font_bold);
    y -= 6.0;
    layer.use_text(data.email(), 11.0, Mm(MARGIN_MM), Mm(y), &font_regular);

    // ── Item table header ─────────────────────────────────────────────
    y -= 14.0;
    let col_qty = MARGIN_MM;
    let col_name = MARGIN_MM + 15.0;
    let col_unit = MARGIN_MM + 115.0;
    let col_line = MARGIN_MM + 150.0;

    layer.use_text("QTY", 10.0, Mm(col_qty), Mm(y), &font_bold);
    layer.use_text("ITEM", 10.0, Mm(col_name), Mm(y), &font_bold);
    layer.use_text("UNIT", 10.0, Mm(col_unit), Mm(y), &font_bold);
    layer.use_text("LINE", 10.0, Mm(col_line), Mm(y), &font_bold);
    y -= 2.0;

    // ── Item rows ─────────────────────────────────────────────────────
    y -= ROW_H_MM;
    let currency = data.currency();
    for item in data.items() {
        layer.use_text(
            format!("{}", item.quantity()),
            10.0,
            Mm(col_qty),
            Mm(y),
            &font_regular,
        );
        layer.use_text(
            truncate(item.name(), 48),
            10.0,
            Mm(col_name),
            Mm(y),
            &font_regular,
        );
        layer.use_text(
            format_money(item.unit_price_cents(), currency),
            10.0,
            Mm(col_unit),
            Mm(y),
            &font_regular,
        );
        layer.use_text(
            format_money(item.line_total_cents(), currency),
            10.0,
            Mm(col_line),
            Mm(y),
            &font_regular,
        );
        y -= ROW_H_MM;
        if y < 60.0 {
            // Defensive: overflow to the bottom of the page. First cut of
            // the generator doesn't paginate; we truncate silently and
            // stop emitting rows. EC-06 follow-up will add multi-page
            // support when we see > ~25-line orders in practice.
            break;
        }
    }

    // ── Totals block ─────────────────────────────────────────────────
    y -= 4.0;
    let totals_label_x = col_unit - 20.0;
    let totals_value_x = col_line;
    let subtotal = data.subtotal_cents();
    let discount = data.discount_cents();
    let tax = data.tax_cents();
    let total = data.total_cents();

    layer.use_text("Subtotal:", 10.0, Mm(totals_label_x), Mm(y), &font_regular);
    layer.use_text(
        format_money(subtotal, currency),
        10.0,
        Mm(totals_value_x),
        Mm(y),
        &font_regular,
    );
    y -= ROW_H_MM;

    if discount > 0 {
        layer.use_text("Discount:", 10.0, Mm(totals_label_x), Mm(y), &font_regular);
        layer.use_text(
            format!("-{}", format_money(discount, currency)),
            10.0,
            Mm(totals_value_x),
            Mm(y),
            &font_regular,
        );
        y -= ROW_H_MM;
    }

    if tax > 0 {
        layer.use_text("Tax:", 10.0, Mm(totals_label_x), Mm(y), &font_regular);
        layer.use_text(
            format_money(tax, currency),
            10.0,
            Mm(totals_value_x),
            Mm(y),
            &font_regular,
        );
        y -= ROW_H_MM;
    }

    layer.use_text("Total:", 11.0, Mm(totals_label_x), Mm(y), &font_bold);
    layer.use_text(
        format_money(total, currency),
        11.0,
        Mm(totals_value_x),
        Mm(y),
        &font_bold,
    );

    // ── Footer ────────────────────────────────────────────────────────
    layer.use_text(footer, 10.0, Mm(MARGIN_MM), Mm(25.0), &font_regular);

    doc.save_to_bytes().unwrap_or_else(|_| fallback_pdf())
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Format integer cents as "$1,234.56" using the currency code as a
/// prefix. We deliberately avoid a locale library; invoices live in a
/// fixed English + 2-digits world.
fn format_money(cents: i64, currency: &str) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let whole = abs / 100;
    let frac = abs % 100;
    let symbol = currency_symbol(currency);
    format!("{sign}{symbol}{whole}.{frac:02}")
}

/// Map an ISO currency code to its single-char symbol, falling back to the
/// uppercase code + space for anything non-common.
fn currency_symbol(code: &str) -> String {
    match code.to_ascii_uppercase().as_str() {
        "USD" | "CAD" | "AUD" | "NZD" | "MXN" => "$".into(),
        "EUR" => "€".into(),
        "GBP" => "£".into(),
        "JPY" | "CNY" => "¥".into(),
        other => format!("{other} "),
    }
}

/// Hard-limit a string to `max` chars (grapheme-counted as bytes — the
/// built-in Helvetica subset we use only supports ASCII anyway, so this
/// is safe in practice). Used to keep item names from overrunning the
/// price column.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut out = String::with_capacity(max);
    for (i, ch) in s.chars().enumerate() {
        if i >= max.saturating_sub(1) {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

/// Absolute fallback: a 1-page empty PDF. Reached only if printpdf's own
/// serialization fails, which it shouldn't on a vanilla builtin-font
/// document. The bytes start with `%PDF-1.3` so downstream type-sniffers
/// still recognise the blob as a PDF.
fn fallback_pdf() -> Vec<u8> {
    // Hand-crafted minimal PDF — 1 blank page. Enough to make `%PDF-` magic
    // hold for the output contract guarantees. Produced once, constant.
    const BLANK: &[u8] = b"%PDF-1.3\n\
%\xe2\xe3\xcf\xd3\n\
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<<>>/Contents 4 0 R>>endobj\n\
4 0 obj<</Length 0>>stream\nendstream\nendobj\n\
xref\n\
0 5\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000052 00000 n \n\
0000000101 00000 n \n\
0000000178 00000 n \n\
trailer<</Size 5/Root 1 0 R>>\n\
startxref\n220\n%%EOF\n";
    BLANK.to_vec()
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Test fixture — a lightweight InvoiceData impl that doesn't depend
    /// on Order / OrderItem so we don't need a DB in the test environment.
    struct Fixture {
        number: String,
        email: String,
        items: Vec<TestItem>,
        subtotal: i64,
        discount: i64,
        tax: i64,
        total: i64,
    }
    struct TestItem {
        name: String,
        qty: i32,
        unit: i64,
        line: i64,
    }
    impl InvoiceItem for TestItem {
        fn name(&self) -> &str {
            &self.name
        }
        fn quantity(&self) -> i32 {
            self.qty
        }
        fn unit_price_cents(&self) -> i64 {
            self.unit
        }
        fn line_total_cents(&self) -> i64 {
            self.line
        }
    }
    impl InvoiceData for Fixture {
        fn number(&self) -> &str {
            &self.number
        }
        fn created_at(&self) -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap()
        }
        fn email(&self) -> &str {
            &self.email
        }
        fn currency(&self) -> &str {
            "USD"
        }
        fn subtotal_cents(&self) -> i64 {
            self.subtotal
        }
        fn discount_cents(&self) -> i64 {
            self.discount
        }
        fn tax_cents(&self) -> i64 {
            self.tax
        }
        fn total_cents(&self) -> i64 {
            self.total
        }
        fn items(&self) -> Box<dyn Iterator<Item = &dyn InvoiceItem> + '_> {
            Box::new(self.items.iter().map(|i| i as &dyn InvoiceItem))
        }
    }

    fn fixture() -> Fixture {
        Fixture {
            number: "ORD-2026-0042".into(),
            email: "buyer@example.com".into(),
            items: vec![
                TestItem {
                    name: "Example product".into(),
                    qty: 1,
                    unit: 1000,
                    line: 1000,
                },
                TestItem {
                    name: "Another product".into(),
                    qty: 2,
                    unit: 500,
                    line: 1000,
                },
            ],
            subtotal: 2000,
            discount: 200,
            tax: 180,
            total: 1980,
        }
    }

    #[test]
    fn invoice_starts_with_pdf_magic_and_is_non_trivial() {
        let bytes = generate_invoice(&fixture());
        assert!(
            bytes.starts_with(b"%PDF-"),
            "bytes should start with %PDF- magic"
        );
        // A real printpdf document is a few KB at minimum. Even the
        // fallback is ~400 bytes; we require more than that so we know
        // printpdf succeeded.
        assert!(bytes.len() > 500, "pdf bytes should be non-trivial");
    }

    #[test]
    fn receipt_starts_with_pdf_magic_and_is_non_trivial() {
        let bytes = generate_receipt(&fixture());
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.len() > 500);
    }

    #[test]
    fn format_money_pads_fraction() {
        assert_eq!(format_money(1234, "USD"), "$12.34");
        assert_eq!(format_money(5, "USD"), "$0.05");
        assert_eq!(format_money(100, "USD"), "$1.00");
        assert_eq!(format_money(-250, "USD"), "-$2.50");
    }

    #[test]
    fn currency_symbol_knows_common_codes() {
        assert_eq!(currency_symbol("USD"), "$");
        assert_eq!(currency_symbol("eur"), "€");
        assert_eq!(currency_symbol("GBP"), "£");
        assert_eq!(currency_symbol("XYZ"), "XYZ ");
    }

    #[test]
    fn truncate_keeps_short_strings_unchanged() {
        assert_eq!(truncate("hello", 20), "hello");
    }

    #[test]
    fn truncate_clips_long_strings() {
        let out = truncate(&"x".repeat(100), 10);
        assert!(out.len() <= 10 * 4); // mul 4 for utf8 upper bound on `…`
        assert!(out.ends_with('…'));
    }

    #[test]
    fn generates_for_empty_item_list() {
        let f = Fixture {
            number: "ORD-2026-9999".into(),
            email: "x@example.com".into(),
            items: vec![],
            subtotal: 0,
            discount: 0,
            tax: 0,
            total: 0,
        };
        let bytes = generate_invoice(&f);
        assert!(bytes.starts_with(b"%PDF-"));
    }
}
