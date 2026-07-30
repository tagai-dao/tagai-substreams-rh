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
ALTER TABLE walnut_nft_pools
    ALTER COLUMN community SET DEFAULT '',
    ALTER COLUMN factory SET DEFAULT '';
ALTER TABLE walnut_nft_batches
    ALTER COLUMN pool SET DEFAULT '',
    ALTER COLUMN batch_id SET DEFAULT 0;
ALTER TABLE walnut_nfts
    ALTER COLUMN pool SET DEFAULT '',
    ALTER COLUMN token_id SET DEFAULT 0;
ALTER TABLE walnut_nft_accounts
    ALTER COLUMN pool SET DEFAULT '',
    ALTER COLUMN account SET DEFAULT '';
ALTER TABLE walnut_basket_tvl_pools
    ALTER COLUMN community SET DEFAULT '',
    ALTER COLUMN factory SET DEFAULT '',
    ALTER COLUMN basket_registry SET DEFAULT '',
    ALTER COLUMN nft_mining_pool SET DEFAULT '',
    ALTER COLUMN nft_reward_bps SET DEFAULT 0,
    ALTER COLUMN lock_duration SET DEFAULT 0;
ALTER TABLE walnut_basket_stakes
    ALTER COLUMN parent_pool SET DEFAULT '',
    ALTER COLUMN basket SET DEFAULT '',
    ALTER COLUMN creator SET DEFAULT '',
    ALTER COLUMN nft_token_id SET DEFAULT 0;
ALTER TABLE walnut_basket_child_pools
    ALTER COLUMN parent_pool SET DEFAULT '',
    ALTER COLUMN basket SET DEFAULT '',
    ALTER COLUMN creator SET DEFAULT '',
    ALTER COLUMN nft_token_id SET DEFAULT 0,
    ALTER COLUMN nft_reward_bps SET DEFAULT 0,
    ALTER COLUMN lock_duration SET DEFAULT 0;
ALTER TABLE walnut_basket_child_positions
    ALTER COLUMN child_pool SET DEFAULT '',
    ALTER COLUMN parent_pool SET DEFAULT '',
    ALTER COLUMN basket SET DEFAULT '',
    ALTER COLUMN account SET DEFAULT '';
COMMIT;
