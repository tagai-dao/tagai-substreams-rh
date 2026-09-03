\set ON_ERROR_STOP on

-- Additive upgrade for an existing RH Substreams PostgreSQL database.
-- Historical discovery rows predate V11 and therefore belong to Pump V9.
BEGIN;

ALTER TABLE pump_token_discoveries
    ADD COLUMN IF NOT EXISTS pump TEXT;
ALTER TABLE pump_token_discoveries
    ADD COLUMN IF NOT EXISTS version INTEGER;

UPDATE pump_token_discoveries
SET pump = '0x6c75e165e52e9c1661a75041650be2d919ee02a1'
WHERE pump IS NULL OR pump = '';

UPDATE pump_token_discoveries
SET version = 9
WHERE version IS NULL OR version = 0;

ALTER TABLE pump_token_discoveries
    ALTER COLUMN pump SET NOT NULL;
ALTER TABLE pump_token_discoveries
    ALTER COLUMN version SET NOT NULL;

ALTER TABLE basket_rebalances
    ADD COLUMN IF NOT EXISTS sell_mask INTEGER NOT NULL DEFAULT 0;
ALTER TABLE basket_rebalances
    ADD COLUMN IF NOT EXISTS buy_mask INTEGER NOT NULL DEFAULT 0;

-- schema.sql uses CREATE ... IF NOT EXISTS for every new V11 and IndexBroker
-- relation. Including it here keeps fresh installs and in-place upgrades on the
-- same canonical schema without deleting or renumbering historical entities.
\ir ../schema.sql

COMMIT;
