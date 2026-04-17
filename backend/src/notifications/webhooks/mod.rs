//! Inbound provider webhook handlers.
//!
//! FDN-05 lands the module shell only — the Resend handler ships with FDN-09
//! when the email transport swaps over. Keeping the sub-module here avoids a
//! re-layout when FDN-09 arrives.

pub mod resend;
