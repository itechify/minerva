-- Characters' portraits from ClassicRank (ADR 0028). Additive only: the Worker before this
-- keeps working until the deploy after.

-- Each Character's portrait as ClassicRank last gave it: asked for when they upload and every
-- day after. It goes when they opt out.
CREATE TABLE portraits (
  -- The Character's name in lower case (characters.name_key).
  name_key TEXT PRIMARY KEY,
  -- Their photo, a PNG on classicrank.com, and their ClassicRank profile: each NULL when
  -- ClassicRank has none. Both NULL, ClassicRank doesn't show them.
  photo TEXT,
  profile TEXT,
  -- When ClassicRank was last asked and answered (ms since the Unix epoch).
  checked_at INTEGER NOT NULL
);
