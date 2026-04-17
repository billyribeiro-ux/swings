-- Migration 052 (POP-03): template library.
--
-- Adds an `is_template` flag to popups so template rows can live alongside
-- live popups in the same table. Seed rows are `is_active=false,
-- is_template=true` and are never served to the public `/api/popups/active`
-- endpoint. Duplicating a template for editing creates a new row with
-- `is_template=false`.
--
-- Each template carries realistic copy in content_json so admins see a
-- working popup the moment they spin it up — no Lorem Ipsum.

ALTER TABLE popups
    ADD COLUMN IF NOT EXISTS is_template BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS popups_is_template_idx ON popups (is_template) WHERE is_template = TRUE;

-- Seed system user for created_by. If the table already has an admin user
-- we reuse whoever was seeded first so the FK is satisfied.
DO $$
DECLARE
    sys_user UUID;
BEGIN
    SELECT id INTO sys_user FROM users ORDER BY created_at ASC LIMIT 1;
    IF sys_user IS NULL THEN
        -- Nothing to seed against. Migration is still valid; templates can
        -- be imported later via admin tooling.
        RETURN;
    END IF;

    -- Guard: do not re-seed templates on repeat runs.
    IF EXISTS (SELECT 1 FROM popups WHERE is_template = TRUE) THEN
        RETURN;
    END IF;

    INSERT INTO popups
        (name, popup_type, trigger_type, trigger_config, content_json,
         style_json, is_active, is_template, priority, created_by)
    VALUES
    -- 1. Newsletter signup
    ('Newsletter signup (starter)', 'modal', 'time_delay',
     '{"delay_ms": 5000}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Get weekly product updates'),
            jsonb_build_object('type', 'paragraph', 'text', 'Join 12,000+ operators reading our Friday digest. Unsubscribe in one click.'),
            jsonb_build_object('type', 'input', 'name', 'email', 'input_type', 'email', 'label', 'Work email', 'required', true),
            jsonb_build_object('type', 'button', 'label', 'Subscribe', 'action', 'submit')
        )
     ),
     '{"background":"#ffffff","textColor":"#111827","accentColor":"#2563eb","maxWidth":"440px"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 2. Exit-intent 10% discount
    ('Exit-intent 10% discount', 'modal', 'exit_intent',
     '{}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Before you go — here is 10% off'),
            jsonb_build_object('type', 'paragraph', 'text', 'Drop your email and we will send a one-time code valid for 24 hours.'),
            jsonb_build_object('type', 'input', 'name', 'email', 'input_type', 'email', 'label', 'Email', 'required', true),
            jsonb_build_object('type', 'button', 'label', 'Email me the code', 'action', 'submit')
        ),
        'discount_code', 'COMEBACK10'
     ),
     '{"background":"#0f172a","textColor":"#f8fafc","accentColor":"#f97316","maxWidth":"460px"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 3. Content locker (email gate)
    ('Content locker (email gate)', 'content_locker', 'on_load',
     '{"lock_selector":".locked-content"}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Unlock the full article'),
            jsonb_build_object('type', 'paragraph', 'text', 'Free. No credit card. One-click unsubscribe.'),
            jsonb_build_object('type', 'input', 'name', 'email', 'input_type', 'email', 'label', 'Email', 'required', true),
            jsonb_build_object('type', 'button', 'label', 'Unlock now', 'action', 'submit')
        ),
        'blur_radius_px', 6
     ),
     '{"background":"rgba(255,255,255,0.92)","textColor":"#111827","accentColor":"#10b981"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 4. Countdown urgency bar
    ('Countdown urgency bar', 'countdown', 'on_load',
     '{"mode":"rolling","duration_secs":900}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'paragraph', 'text', 'Spring sale ends in'),
            jsonb_build_object('type', 'countdown_timer', 'format', 'HH:MM:SS'),
            jsonb_build_object('type', 'button', 'label', 'Shop now', 'action', 'redirect', 'url', '/sale')
        )
     ),
     '{"background":"#dc2626","textColor":"#ffffff","position":"top","height":"48px"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 5. Spin-to-win
    ('Spin-to-win discount wheel', 'spin_to_win', 'time_delay',
     '{"delay_ms": 8000}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Spin to win your discount'),
            jsonb_build_object('type', 'input', 'name', 'email', 'input_type', 'email', 'label', 'Email', 'required', true),
            jsonb_build_object('type', 'spin_wheel', 'prizes', jsonb_build_array(
                jsonb_build_object('label', '5% off',  'weight', 40, 'coupon_code_pattern', 'SPIN5-{{nanoid:8}}'),
                jsonb_build_object('label', '10% off', 'weight', 30, 'coupon_code_pattern', 'SPIN10-{{nanoid:8}}'),
                jsonb_build_object('label', '15% off', 'weight', 20, 'coupon_code_pattern', 'SPIN15-{{nanoid:8}}'),
                jsonb_build_object('label', 'Free shipping', 'weight', 9, 'coupon_code_pattern', 'SPINFS-{{nanoid:8}}'),
                jsonb_build_object('label', '25% off JACKPOT', 'weight', 1, 'coupon_code_pattern', 'SPIN25-{{nanoid:8}}')
            )),
            jsonb_build_object('type', 'button', 'label', 'Spin', 'action', 'submit')
        )
     ),
     '{"background":"#0b1120","textColor":"#f1f5f9","accentColor":"#facc15","maxWidth":"560px"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 6. Scratch-card reveal
    ('Scratch-card reveal', 'scratch_card', 'time_delay',
     '{"delay_ms": 6000}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Scratch to reveal your offer'),
            jsonb_build_object('type', 'scratch_card', 'prize_label', '$15 credit on your first order', 'coupon_code', 'SCRATCH15'),
            jsonb_build_object('type', 'input', 'name', 'email', 'input_type', 'email', 'label', 'Email to claim', 'required', true),
            jsonb_build_object('type', 'button', 'label', 'Claim prize', 'action', 'submit')
        )
     ),
     '{"background":"#fff7ed","textColor":"#1f2937","accentColor":"#ea580c"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 7. NPS feedback survey
    ('NPS feedback survey', 'slide_in', 'scroll_percentage',
     '{"percentage": 75}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Quick question'),
            jsonb_build_object('type', 'paragraph', 'text', 'How likely are you to recommend us to a colleague?'),
            jsonb_build_object('type', 'rating_scale', 'name', 'nps', 'min', 0, 'max', 10, 'label_min', 'Not likely', 'label_max', 'Extremely likely', 'required', true),
            jsonb_build_object('type', 'textarea', 'name', 'reason', 'label', 'What is the main reason for your score?'),
            jsonb_build_object('type', 'button', 'label', 'Submit', 'action', 'submit')
        )
     ),
     '{"background":"#ffffff","textColor":"#111827","accentColor":"#6366f1","position":"bottom-right"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 8. Product survey (branching)
    ('Product survey with branching', 'modal', 'manual',
     '{}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Help us build the right features'),
            jsonb_build_object('type', 'radio', 'name', 'primary_role', 'label', 'What is your primary role?', 'options', jsonb_build_array('Engineer','Product','Marketing','Founder','Other'), 'required', true),
            jsonb_build_object('type', 'page_break'),
            jsonb_build_object('type', 'checkbox_group', 'name', 'priorities', 'label', 'What would you like to see next? (pick up to 3)', 'max_selected', 3, 'options', jsonb_build_array('Better analytics','Integrations','Mobile app','AI copilots','Custom workflows')),
            jsonb_build_object('type', 'button', 'label', 'Finish survey', 'action', 'submit')
        )
     ),
     '{"background":"#ffffff","textColor":"#0f172a","accentColor":"#0d9488"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 9. Lead magnet PDF download
    ('Lead magnet PDF download', 'modal', 'scroll_percentage',
     '{"percentage": 60}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Download the playbook (PDF)'),
            jsonb_build_object('type', 'paragraph', 'text', 'The 22-page guide our top customers use to onboard their first 100 users.'),
            jsonb_build_object('type', 'input', 'name', 'email', 'input_type', 'email', 'label', 'Work email', 'required', true),
            jsonb_build_object('type', 'input', 'name', 'company', 'input_type', 'text', 'label', 'Company'),
            jsonb_build_object('type', 'button', 'label', 'Send me the PDF', 'action', 'submit')
        ),
        'download_url', '/downloads/onboarding-playbook.pdf'
     ),
     '{"background":"#f8fafc","textColor":"#0f172a","accentColor":"#2563eb"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 10. Announcement bar
    ('Site-wide announcement bar', 'floating_bar', 'on_load',
     '{}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'paragraph', 'text', 'New: we just launched self-serve SSO — read the changelog'),
            jsonb_build_object('type', 'button', 'label', 'Read more', 'action', 'redirect', 'url', '/changelog')
        )
     ),
     '{"background":"#1e293b","textColor":"#f8fafc","position":"top"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 11. Cookie pre-banner teaser
    ('Cookie pre-banner teaser', 'notification', 'on_load',
     '{}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'paragraph', 'text', 'We use cookies to improve your experience.'),
            jsonb_build_object('type', 'button', 'label', 'Customize', 'action', 'open_preferences'),
            jsonb_build_object('type', 'button', 'label', 'Accept all', 'action', 'consent_accept_all')
        )
     ),
     '{"background":"#111827","textColor":"#f9fafb","accentColor":"#22c55e","position":"bottom-left"}'::jsonb,
     FALSE, TRUE, 0, sys_user),

    -- 12. Session feedback prompt
    ('Session feedback prompt', 'slide_in', 'inactivity',
     '{"idle_secs": 90}'::jsonb,
     jsonb_build_object(
        'elements', jsonb_build_array(
            jsonb_build_object('type', 'heading', 'text', 'Finding what you need?'),
            jsonb_build_object('type', 'radio', 'name', 'success', 'label', 'Did you find what you were looking for?', 'options', jsonb_build_array('Yes, thanks','Not yet','Not sure'), 'required', true),
            jsonb_build_object('type', 'textarea', 'name', 'detail', 'label', 'Anything we can help with?'),
            jsonb_build_object('type', 'button', 'label', 'Send feedback', 'action', 'submit')
        )
     ),
     '{"background":"#ffffff","textColor":"#111827","accentColor":"#0ea5e9","position":"bottom-right"}'::jsonb,
     FALSE, TRUE, 0, sys_user);
END $$;
