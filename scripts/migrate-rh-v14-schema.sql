\set ON_ERROR_STOP on
BEGIN;
-- RH V14 additive schema. Frozen legacy modules keep their table contracts.
CREATE TABLE IF NOT EXISTS v14_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT '',
    payload TEXT NOT NULL DEFAULT '',
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS v14_events_chain_idx ON v14_events (block_number, log_index, id);
CREATE INDEX IF NOT EXISTS v14_events_source_idx ON v14_events (source, event_type, block_number, log_index);
CREATE TABLE IF NOT EXISTS token_index_configs (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL DEFAULT '',
    entity_index BIGINT NOT NULL DEFAULT 0,
    name TEXT NOT NULL DEFAULT '',
    symbol TEXT NOT NULL DEFAULT '',
    basket_fee_bps BIGINT NOT NULL DEFAULT 0,
    creator_share_bps BIGINT NOT NULL DEFAULT 0,
    retain_community_ownership BOOLEAN NOT NULL DEFAULT FALSE,
    component_count BIGINT NOT NULL DEFAULT 0,
    community TEXT NOT NULL DEFAULT '',
    index_token TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS token_components (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL DEFAULT '',
    asset TEXT NOT NULL DEFAULT '',
    position BIGINT NOT NULL DEFAULT 0,
    target_weight BIGINT NOT NULL DEFAULT 0,
    pair TEXT NOT NULL DEFAULT '',
    staking_pool TEXT NOT NULL DEFAULT '',
    staking_reward_ratio BIGINT NOT NULL DEFAULT 0,
    initial_native_budget NUMERIC(78,0) NOT NULL DEFAULT 0,
    initial_token_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    initial_asset_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    initial_lp_to_dead NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_tax_to_dead NUMERIC(78,0) NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS token_components_token_idx ON token_components (token, position);
CREATE TABLE IF NOT EXISTS token_buyback_states (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL DEFAULT '',
    index_token TEXT NOT NULL DEFAULT '',
    native_reserve NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_buyback_fee_native NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_native_spent NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_bought NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_notified NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    acc_reward_per_token NUMERIC(78,0) NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS token_optional_pools (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL DEFAULT '',
    pool TEXT NOT NULL DEFAULT '',
    factory TEXT NOT NULL DEFAULT '',
    reward_ratio BIGINT NOT NULL DEFAULT 0,
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS pump_optional_pool_factories (
    id TEXT PRIMARY KEY,
    pump TEXT NOT NULL DEFAULT '',
    factory TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    max_reward_ratio BIGINT NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS pump_constituents (
    id TEXT PRIMARY KEY,
    pump TEXT NOT NULL DEFAULT '',
    asset TEXT NOT NULL DEFAULT '',
    approved BOOLEAN NOT NULL DEFAULT FALSE,
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS token_hook_periods (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL DEFAULT '',
    period_index BIGINT NOT NULL DEFAULT 0,
    period_volume NUMERIC(78,0) NOT NULL DEFAULT 0,
    lookup_volume NUMERIC(78,0) NOT NULL DEFAULT 0,
    ratio_ppm BIGINT NOT NULL DEFAULT 0,
    inject_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS v14_router_price_pools (
    id TEXT PRIMARY KEY,
    router TEXT NOT NULL DEFAULT '',
    pool_id TEXT NOT NULL DEFAULT '',
    token0 TEXT NOT NULL DEFAULT '',
    token1 TEXT NOT NULL DEFAULT '',
    source_type BIGINT NOT NULL DEFAULT 0,
    source_data TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT FALSE,
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS v14_router_routes (
    id TEXT PRIMARY KEY,
    router TEXT NOT NULL DEFAULT '',
    token0 TEXT NOT NULL DEFAULT '',
    token1 TEXT NOT NULL DEFAULT '',
    route_hash TEXT NOT NULL DEFAULT '',
    pool_ids TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT FALSE,
    block_number BIGINT NOT NULL DEFAULT 0,
    block_hash TEXT NOT NULL DEFAULT '',
    block_timestamp BIGINT NOT NULL DEFAULT 0,
    transaction_hash TEXT NOT NULL DEFAULT '',
    log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS v14_trade_memberships (
    id TEXT PRIMARY KEY,
    pool TEXT NOT NULL DEFAULT '',
    community TEXT NOT NULL DEFAULT '',
    account TEXT NOT NULL DEFAULT '',
    created_at BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS v14_trade_memberships_community_idx ON v14_trade_memberships (community, account);
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS ipshare_subject TEXT NOT NULL DEFAULT '';
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS listing_status TEXT NOT NULL DEFAULT '';
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS listing_state_block BIGINT NOT NULL DEFAULT 0;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS listing_state_log_index BIGINT NOT NULL DEFAULT 0;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS listing_queued_at BIGINT NOT NULL DEFAULT 0;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS listed_at BIGINT NOT NULL DEFAULT 0;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS price_updated_block BIGINT NOT NULL DEFAULT 0;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS price_updated_log_index BIGINT NOT NULL DEFAULT 0;
ALTER TABLE token_trade_events ADD COLUMN IF NOT EXISTS venue TEXT NOT NULL DEFAULT '';
ALTER TABLE token_trade_events ADD COLUMN IF NOT EXISTS pool TEXT NOT NULL DEFAULT '';
ALTER TABLE token_trade_events ADD COLUMN IF NOT EXISTS buyback_fee NUMERIC(78,0) NOT NULL DEFAULT 0;
ALTER TABLE pairs ADD COLUMN IF NOT EXISTS venue TEXT NOT NULL DEFAULT '';
ALTER TABLE pairs ADD COLUMN IF NOT EXISTS quote_asset TEXT NOT NULL DEFAULT '';
ALTER TABLE pairs ADD COLUMN IF NOT EXISTS component TEXT NOT NULL DEFAULT '';
ALTER TABLE pairs ADD COLUMN IF NOT EXISTS staking_pool TEXT NOT NULL DEFAULT '';
ALTER TABLE pairs ADD COLUMN IF NOT EXISTS staking_reward_ratio BIGINT NOT NULL DEFAULT 0;
ALTER TABLE baskets ADD COLUMN IF NOT EXISTS source_token TEXT NOT NULL DEFAULT '';
ALTER TABLE walnut_pools ADD COLUMN IF NOT EXISTS total_claimed NUMERIC(78,0) NOT NULL DEFAULT 0;
ALTER TABLE walnut_operations ADD COLUMN IF NOT EXISTS trade_order_id NUMERIC(78,0) NOT NULL DEFAULT 0;
ALTER TABLE walnut_operations ADD COLUMN IF NOT EXISTS trade_harvested BOOLEAN NOT NULL DEFAULT FALSE;

-- Independent V14 membership stores must never increment the legacy users_count:
-- a future legacy action by the same user would increment it a second time.
CREATE SCHEMA IF NOT EXISTS rh_read;
CREATE OR REPLACE VIEW rh_read.walnut_account_communities AS
SELECT account, community, MIN(created_at) AS created_at FROM (
 SELECT account, community, created_at FROM walnut_account_communities
 UNION ALL SELECT account, community, created_at FROM v14_trade_memberships
) AS members GROUP BY account, community;
CREATE OR REPLACE VIEW rh_read.walnut_account_pools AS
SELECT account, pool, MIN(created_at) AS created_at FROM (
 SELECT account, pool, created_at FROM walnut_account_pools
 UNION ALL SELECT account, pool, created_at FROM v14_trade_memberships
) AS members GROUP BY account, pool;


-- Legacy Community ratio events can bootstrap multiple new trade pools before
-- the new factory writer assigns their positive deterministic indexes.
ALTER TABLE walnut_pools DROP CONSTRAINT IF EXISTS walnut_pools_entity_index_key;
CREATE UNIQUE INDEX IF NOT EXISTS walnut_pools_positive_entity_index_idx
    ON walnut_pools (entity_index) WHERE entity_index > 0;

ALTER TABLE pairs ALTER COLUMN token_index SET DEFAULT 0;
ALTER TABLE pairs ALTER COLUMN token SET DEFAULT '';

COMMIT;
