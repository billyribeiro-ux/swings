-- Custom enum types
CREATE TYPE user_role AS ENUM ('member', 'admin');
CREATE TYPE subscription_plan AS ENUM ('monthly', 'annual');
CREATE TYPE subscription_status AS ENUM ('active', 'canceled', 'past_due', 'trialing', 'unpaid');
CREATE TYPE trade_direction AS ENUM ('bullish', 'bearish');

-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    role user_role NOT NULL DEFAULT 'member',
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users (email);

-- Refresh tokens
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens (token_hash);
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);

-- Subscriptions
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stripe_customer_id TEXT NOT NULL,
    stripe_subscription_id TEXT NOT NULL UNIQUE,
    plan subscription_plan NOT NULL,
    status subscription_status NOT NULL DEFAULT 'active',
    current_period_start TIMESTAMPTZ NOT NULL,
    current_period_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_user ON subscriptions (user_id);
CREATE INDEX idx_subscriptions_stripe ON subscriptions (stripe_subscription_id);
CREATE INDEX idx_subscriptions_customer ON subscriptions (stripe_customer_id);

-- Watchlists
CREATE TABLE watchlists (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    week_of DATE NOT NULL,
    video_url TEXT,
    notes TEXT,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_watchlists_week ON watchlists (week_of DESC);

-- Watchlist alerts
CREATE TABLE watchlist_alerts (
    id UUID PRIMARY KEY,
    watchlist_id UUID NOT NULL REFERENCES watchlists(id) ON DELETE CASCADE,
    ticker TEXT NOT NULL,
    direction trade_direction NOT NULL DEFAULT 'bullish',
    entry_zone TEXT NOT NULL,
    invalidation TEXT NOT NULL,
    profit_zones TEXT[] NOT NULL DEFAULT '{}',
    notes TEXT,
    chart_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerts_watchlist ON watchlist_alerts (watchlist_id);

-- Course enrollments
CREATE TABLE course_enrollments (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    course_id TEXT NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE(user_id, course_id)
);

CREATE INDEX idx_enrollments_user ON course_enrollments (user_id);
