-- Emails are compared case-insensitively in the app; store canonical lowercase form
-- so UNIQUE(email) matches real-world addresses (e.g. Gmail).
UPDATE users SET email = lower(btrim(email));
