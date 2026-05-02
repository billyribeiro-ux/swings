-- 084_subscription_invoice_urls.sql
--
-- Adds the Stripe-hosted invoice URLs (`hosted_invoice_url`,
-- `invoice_pdf`) to the `subscription_invoices` mirror so the member
-- account section can render "View Receipt" + "Download PDF" links
-- without round-tripping to Stripe on every dashboard render.
--
-- Both fields are top-level on the Stripe Invoice object and are
-- populated as soon as Stripe finalizes the invoice (i.e. they exist by
-- the time the `invoice.paid` / `invoice.payment_failed` webhooks fire).

ALTER TABLE subscription_invoices
    ADD COLUMN IF NOT EXISTS hosted_invoice_url TEXT,
    ADD COLUMN IF NOT EXISTS invoice_pdf        TEXT;
