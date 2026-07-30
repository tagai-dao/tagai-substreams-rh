\set ON_ERROR_STOP on

-- schema.sql uses CREATE TABLE/INDEX IF NOT EXISTS throughout. Reusing it
-- keeps fresh sink setup and production upgrades on the exact same schema.
BEGIN;
\ir ../schema.sql
ALTER TABLE walnut_nft_pools
    ADD COLUMN IF NOT EXISTS updated_block BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS updated_at BIGINT NOT NULL DEFAULT 0;
ALTER TABLE walnut_nft_batches
    ADD COLUMN IF NOT EXISTS updated_block BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS updated_at BIGINT NOT NULL DEFAULT 0;
ALTER TABLE walnut_basket_tvl_pools
    ADD COLUMN IF NOT EXISTS updated_block BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS updated_at BIGINT NOT NULL DEFAULT 0;
ALTER TABLE walnut_basket_stakes
    ADD COLUMN IF NOT EXISTS updated_at BIGINT NOT NULL DEFAULT 0;
ALTER TABLE walnut_basket_child_pools
    ADD COLUMN IF NOT EXISTS updated_block BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS updated_at BIGINT NOT NULL DEFAULT 0;
COMMIT;
