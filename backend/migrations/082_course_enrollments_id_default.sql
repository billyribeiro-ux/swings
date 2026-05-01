-- Add DEFAULT gen_random_uuid() to course_enrollments.id.
--
-- The column was originally created in `001_initial.sql` as plain
-- `id UUID PRIMARY KEY` — no default. Every INSERT into the table had
-- to supply the id explicitly. The handler in
-- `backend/src/handlers/courses.rs::enroll_course` did not, which
-- caused the endpoint to 500 on the first POST with
-- `null value in column "id" of relation "course_enrollments" violates
-- not-null constraint`. Phase A's course-enrollment gate change
-- exercised the path for the first time end-to-end and Phase C's real
-- Stripe E2E surfaced the failure (see Phase C scenario 11).
--
-- Fix: add a DEFAULT so the column behaves like every other UUID PK in
-- the schema. The handler is also updated to bind an explicit
-- `Uuid::new_v4()` for belt-and-braces — both forms now succeed.
ALTER TABLE course_enrollments
    ALTER COLUMN id SET DEFAULT gen_random_uuid();
