//! EC — e-commerce domain module (digital-goods trim).
//!
//! Subsystem scope (per AUDIT_PHASE3_PLAN §0 D1):
//!   * **EC-01** — product model: simple + subscription + downloadable +
//!     bundle types, variants, downloadable assets, bundle items.
//!   * **EC-03** — [`cart`]: persistent guest + authed cart with post-login
//!     merge. Totals computed in-memory from product + coupon state.
//!   * **EC-06** ([`invoice`]) — printpdf-based invoice / receipt / credit-note
//!     generator with a hard-coded US Letter layout. Stored locally under
//!     `uploads/invoices/` for now; R2 migration is a later concern.
//!   * **EC-11** — [`coupons`] engine refactor: `Money`-based math, BOGO,
//!     recurring-vs-one-time flagging, category + product scope. Delegated to
//!     from [`crate::handlers::coupons`] without changing the public HTTP surface.
//!   * Later tickets (EC-02, EC-04..EC-05, EC-07..EC-10, EC-12) layer catalog,
//!     checkout, orders, digital delivery, tax, subscriptions, memberships, and
//!     reports on top of the foundations established here.
//!
//! Money handling: every currency field is stored as an `i64` of minor units
//! (`BIGINT` cents on the DB side). Row structs mirror this `i64` shape so
//! `sqlx::FromRow` can decode transparently; the [`common::money::Money`]
//! newtype is applied in the service + handler layers, never in the row layer.

pub mod cart;
pub mod coupons;
pub mod products;
pub mod repo;

pub use cart::{Cart, CartIdentity, CartItem, CartTotals};
pub use coupons::{
    AppliedCoupon, BogoConfig, CartLine, CouponEngine, CouponInput, CouponScope, RecurringMode,
};
pub use products::{
    BundleItem, DownloadableAsset, Product, ProductStatus, ProductType, ProductVariant,
};
