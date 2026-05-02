-- Adds currency + order_id to coupon_usages so the member coupons history
-- page can show "got $5 off USD" + a link to the order that got the discount.
--
-- currency defaults to 'usd' for back-compat with existing rows; order_id is
-- nullable because a coupon can be redeemed against either a subscription or
-- an order (or both — the discount applies once to the parent invoice).

ALTER TABLE coupon_usages
    ADD COLUMN IF NOT EXISTS currency TEXT NOT NULL DEFAULT 'usd',
    ADD COLUMN IF NOT EXISTS order_id UUID REFERENCES orders(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_coupon_usages_order ON coupon_usages (order_id)
    WHERE order_id IS NOT NULL;
