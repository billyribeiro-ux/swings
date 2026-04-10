-- Analytics: client sessions and events (page views, impressions, clicks for CTR)

CREATE TABLE analytics_sessions (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_analytics_sessions_user ON analytics_sessions (user_id);

CREATE TABLE analytics_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES analytics_sessions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '/',
    referrer TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT analytics_events_type_chk CHECK (
        event_type IN ('page_view', 'impression', 'click')
    ),
    CONSTRAINT analytics_events_path_len CHECK (char_length(path) <= 2048)
);

CREATE INDEX idx_analytics_events_created ON analytics_events (created_at DESC);
CREATE INDEX idx_analytics_events_session ON analytics_events (session_id);
CREATE INDEX idx_analytics_events_type_time ON analytics_events (event_type, created_at DESC);
CREATE INDEX idx_analytics_events_path ON analytics_events (path);
CREATE INDEX idx_analytics_events_metadata_gin ON analytics_events USING GIN (metadata);
