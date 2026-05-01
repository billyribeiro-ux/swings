//! EC — e-commerce domain module (digital-goods trim).
//!
//! Subsystem scope:
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

pub mod billing;
pub mod cart;
pub mod catalog;
pub mod checkout;
pub mod coupons;
pub mod disputes;
pub mod downloads;
pub mod memberships;
pub mod orders;
pub mod products;
pub mod refunds;
pub mod repo;
pub mod reports;
pub mod subscriptions;
pub mod tax;
pub mod webhook_audit;

pub use reports::{
    churn_rate, cohort_ltv, coupon_performance, lifetime_value_for_user, mrr_arr, revenue_summary,
    ChurnRate, CohortPoint, CouponPerformance, MrrArr, RevenueSummary,
};

pub use memberships::{
    can_access, cancel_membership, create_plan as create_membership_plan, grant_membership,
    list_user_memberships, plan_grants, Membership, MembershipPlan, Resource,
};

pub use subscriptions::{
    next_dunning_attempt, pause as pause_subscription, prorate, record_change as record_sub_change,
    record_dunning_result, resume as resume_subscription, schedule_dunning, DunningAttempt,
    SubscriptionChange, SubscriptionError,
};

pub use checkout::{
    create_checkout_session, delete_address, list_addresses, save_address, Address, CheckoutError,
    CheckoutSession, MintedIntent, StripeIntentMinter, UpsertAddress,
};

pub use cart::{Cart, CartIdentity, CartItem, CartTotals};
pub use coupons::{
    AppliedCoupon, BogoConfig, CartLine, CouponEngine, CouponInput, CouponScope, RecurringMode,
};
pub use orders::{
    can_transition, create_order, get_order, get_order_by_payment_intent, next_order_number,
    transition, CreateOrderInput, Order, OrderError, OrderItem, OrderNote, OrderRefund,
    OrderStatus,
};
pub use products::{
    BundleItem, DownloadableAsset, Product, ProductStatus, ProductType, ProductVariant,
};
