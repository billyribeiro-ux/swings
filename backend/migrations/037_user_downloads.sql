-- EC-07: Per-user download grants for digital delivery.
--
-- A `user_downloads` row represents a single purchase's entitlement to pull
-- a specific asset. It is created by the `digital_delivery` outbox handler
-- on `order.completed` and consumed by `GET /api/downloads/{token}` which
-- trades the token for a short-lived signed R2 URL.
--
-- The token column is the raw 32-byte random issued at grant time. We
-- store the raw bytes (not a hash) to match the pattern `forms/uploads.rs`
-- uses — the token is delivered to the user over TLS and scoped to their
-- account; treating it as a bearer credential keeps the flow simple and
-- the unique-index lookup cheap. GDPR user-deletion cascades via
-- `ON DELETE CASCADE` on the FK.
--
-- `asset_id` is a free UUID rather than a hard FK to `downloadable_assets`
-- so the grant row survives an admin soft-deleting the asset after a
-- purchase — the storage key lives on the row itself so we can honour the
-- grant even if the asset row is gone. The worker verifies the asset
-- exists at grant-time; after that the grant is self-contained.

CREATE TABLE IF NOT EXISTS user_downloads (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    order_id            UUID REFERENCES orders(id) ON DELETE SET NULL,
    product_id          UUID NOT NULL REFERENCES products(id),
    asset_id            UUID NOT NULL,
    storage_key         TEXT NOT NULL,
    mime_type           TEXT NOT NULL DEFAULT 'application/octet-stream',
    download_token      BYTEA NOT NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    downloads_remaining INT NOT NULL DEFAULT 5 CHECK (downloads_remaining >= 0),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS user_downloads_token_idx ON user_downloads (download_token);
CREATE INDEX IF NOT EXISTS user_downloads_user_idx ON user_downloads (user_id);
CREATE INDEX IF NOT EXISTS user_downloads_order_idx ON user_downloads (order_id)
    WHERE order_id IS NOT NULL;
