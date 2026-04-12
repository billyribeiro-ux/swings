-- Migration 015: Full popup/form builder system with submissions and event tracking

CREATE TABLE popups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    popup_type TEXT NOT NULL DEFAULT 'modal' CHECK (popup_type IN ('modal', 'slide_in', 'banner', 'fullscreen', 'floating_bar', 'inline')),
    trigger_type TEXT NOT NULL DEFAULT 'time_delay' CHECK (trigger_type IN ('on_load', 'exit_intent', 'scroll_percentage', 'time_delay', 'click', 'manual', 'inactivity')),
    trigger_config JSONB NOT NULL DEFAULT '{"delay_ms": 3000}',
    content_json JSONB NOT NULL DEFAULT '{"elements": []}',
    style_json JSONB NOT NULL DEFAULT '{"background": "#1a1a2e", "textColor": "#ffffff", "accentColor": "#0fa4af", "borderRadius": "16px", "maxWidth": "480px", "animation": "fade", "backdrop": true, "backdropColor": "rgba(0,0,0,0.6)"}',
    targeting_rules JSONB NOT NULL DEFAULT '{"pages": ["*"], "devices": ["desktop", "mobile", "tablet"], "userStatus": ["all"]}',
    display_frequency TEXT NOT NULL DEFAULT 'once_per_session' CHECK (display_frequency IN ('every_time', 'once_per_session', 'once_ever', 'custom')),
    frequency_config JSONB NOT NULL DEFAULT '{}',
    success_message TEXT DEFAULT 'Thank you!',
    redirect_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    starts_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    priority INTEGER NOT NULL DEFAULT 0,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popups_active ON popups (is_active, priority DESC);

CREATE TABLE popup_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    session_id UUID,
    form_data JSONB NOT NULL DEFAULT '{}',
    ip_address TEXT,
    user_agent TEXT,
    page_url TEXT,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popup_submissions_popup ON popup_submissions (popup_id, submitted_at DESC);
CREATE INDEX idx_popup_submissions_user ON popup_submissions (user_id);

CREATE TABLE popup_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('impression', 'close', 'submit', 'click')),
    session_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popup_events_popup ON popup_events (popup_id, event_type, created_at DESC);
