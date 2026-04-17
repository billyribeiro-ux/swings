//! EC — e-commerce domain module (digital-goods trim).
//!
//! Subsystem scope (per AUDIT_PHASE3_PLAN §0 D1):
//!   * **EC-01** (this bootstrap) — product model: simple + subscription +
//!     downloadable + bundle types, variants, downloadable assets, bundle items.
//!   * **EC-11** — [`coupons`] engine refactor: `Money`-based math, BOGO,
//!     recurring-vs-one-time flagging, category + product scope. Delegated to
//!     from [`crate::handlers::coupons`] without changing the public HTTP surface.
//!   * Later tickets (EC-02..EC-10, EC-12) layer catalog, cart, checkout,
//!     orders, and reports on top of the foundations established here.
//!
//! Money handling: every currency field is stored as an `i64` of minor units
//! (`BIGINT` cents on the DB side). Row structs mirror this `i64` shape so
//! `sqlx::FromRow` can decode transparently; the [`common::money::Money`]
//! newtype is applied in the service + handler layers, never in the row layer.

pub mod coupons;
pub mod products;
pub mod repo;

pub use coupons::{
    AppliedCoupon, BogoConfig, CartLine, CouponEngine, CouponInput, CouponScope, RecurringMode,
};
pub use products::{
    BundleItem, DownloadableAsset, Product, ProductStatus, ProductType, ProductVariant,
};
