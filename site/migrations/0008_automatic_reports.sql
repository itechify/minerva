-- Automatic reports (ADR 0032): the Worker reports an entry far above a busy map's #1 by
-- itself. Additive only: the Worker before this keeps working until the deploy after.

-- 1 for a report the Worker made at upload, 0 for a player's.
ALTER TABLE reports ADD COLUMN automatic INTEGER NOT NULL DEFAULT 0;
