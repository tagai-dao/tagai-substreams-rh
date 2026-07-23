CREATE TABLE IF NOT EXISTS pump_token_discoveries (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    creator TEXT NOT NULL,
    symbol TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE INDEX IF NOT EXISTS pump_token_discoveries_creator_idx
    ON pump_token_discoveries (creator);

CREATE INDEX IF NOT EXISTS pump_token_discoveries_block_idx
    ON pump_token_discoveries (block_number, log_index);

CREATE TABLE IF NOT EXISTS tokens (
    id TEXT PRIMARY KEY,
    -- Mutable entities are written as PostgreSQL upserts. PostgreSQL validates
    -- NOT NULL columns on the INSERT candidate before resolving ON CONFLICT, so
    -- every column omitted by a partial update needs a safe bootstrap default.
    entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    symbol TEXT NOT NULL DEFAULT '',
    creator TEXT NOT NULL DEFAULT '',
    pump TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 0,
    listed BOOLEAN NOT NULL DEFAULT FALSE,
    buy_times BIGINT NOT NULL DEFAULT 0,
    sell_times BIGINT NOT NULL DEFAULT 0,
    holders_count BIGINT NOT NULL DEFAULT 0,
    tiptag_fee NUMERIC(78, 0) NOT NULL DEFAULT 0,
    sellsman_fee NUMERIC(78, 0) NOT NULL DEFAULT 0,
    bonding_curve_supply NUMERIC(78, 0) NOT NULL DEFAULT 0,
    max_bonding_curve_supply NUMERIC(78, 0) NOT NULL DEFAULT 0,
    price NUMERIC(78, 0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_log_index INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS tokens_creator_idx ON tokens (creator);
CREATE INDEX IF NOT EXISTS tokens_creation_idx
    ON tokens (creation_block, creation_log_index);

CREATE TABLE IF NOT EXISTS token_trade_events (
    id TEXT PRIMARY KEY,
    entity_index BIGINT NOT NULL UNIQUE,
    token TEXT NOT NULL,
    buyer TEXT NOT NULL,
    sellsman TEXT NOT NULL,
    is_buy BOOLEAN NOT NULL,
    token_amount NUMERIC(78, 0) NOT NULL,
    eth_amount NUMERIC(78, 0) NOT NULL,
    tiptag_fee NUMERIC(78, 0) NOT NULL,
    sellsman_fee NUMERIC(78, 0) NOT NULL,
    price NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE INDEX IF NOT EXISTS token_trade_events_token_time_idx
    ON token_trade_events (token, block_timestamp DESC, log_index DESC);

CREATE TABLE IF NOT EXISTS token_transfer_events (
    id TEXT PRIMARY KEY,
    entity_index BIGINT UNIQUE,
    token TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    amount NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE INDEX IF NOT EXISTS token_transfer_events_token_time_idx
    ON token_transfer_events (token, block_timestamp DESC, log_index DESC);

CREATE TABLE IF NOT EXISTS token_listings (
    token TEXT PRIMARY KEY,
    entity_index BIGINT NOT NULL UNIQUE,
    event_token TEXT NOT NULL,
    pool_id TEXT NOT NULL UNIQUE,
    sqrt_price_x96 NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

-- Replacement Basket protocol (RH chain, indexed from block 16,303,863). Constituent
-- composition and live reserves remain chain-read data and are intentionally
-- not duplicated here.
CREATE TABLE IF NOT EXISTS baskets (
    id TEXT PRIMARY KEY,
    -- Trade blocks update only aggregate columns. Defaults are required on
    -- creation fields because PostgreSQL checks the INSERT candidate before
    -- applying ON CONFLICT to an existing Basket row.
    creator TEXT NOT NULL DEFAULT '',
    registrar TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0,
    salt TEXT NOT NULL DEFAULT '0x',
    buy_count BIGINT NOT NULL DEFAULT 0,
    sell_count BIGINT NOT NULL DEFAULT 0,
    total_usdg_volume NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_fee_weth NUMERIC(78, 0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_block_hash TEXT NOT NULL DEFAULT '',
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    UNIQUE (creation_transaction_hash, creation_log_index)
);
-- Keep schema.sql safe for databases initialized before partial Basket
-- aggregate upserts received their required bootstrap defaults.
ALTER TABLE baskets ALTER COLUMN creator SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN registrar SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE baskets ALTER COLUMN created_at SET DEFAULT 0;
ALTER TABLE baskets ALTER COLUMN creation_block SET DEFAULT 0;
ALTER TABLE baskets ALTER COLUMN creation_block_hash SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN creation_transaction_hash SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN creation_log_index SET DEFAULT 0;
CREATE INDEX IF NOT EXISTS baskets_creator_idx ON baskets (creator);
CREATE INDEX IF NOT EXISTS baskets_creation_idx ON baskets (creation_block, creation_log_index);

CREATE TABLE IF NOT EXISTS basket_trade_events (
    id TEXT PRIMARY KEY,
    basket TEXT NOT NULL,
    is_buy BOOLEAN NOT NULL,
    payer TEXT NOT NULL,
    recipient TEXT,
    frontend TEXT NOT NULL,
    usdg_amount NUMERIC(78, 0) NOT NULL,
    basket_amount NUMERIC(78, 0) NOT NULL,
    fee_weth NUMERIC(78, 0) NOT NULL,
    routed BOOLEAN NOT NULL,
    router_log_index INTEGER,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS basket_trade_events_basket_time_idx
    ON basket_trade_events (basket, block_timestamp DESC, log_index DESC);
CREATE INDEX IF NOT EXISTS basket_trade_events_payer_time_idx
    ON basket_trade_events (payer, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS basket_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,
    basket TEXT NOT NULL,
    account TEXT,
    recipient TEXT,
    asset TEXT,
    amount NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS basket_operations_basket_time_idx
    ON basket_operations (basket, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS basket_fee_accrual_events (
    id TEXT PRIMARY KEY,
    basket TEXT NOT NULL,
    holder_amount NUMERIC(78, 0) NOT NULL,
    auction_amount NUMERIC(78, 0) NOT NULL,
    creator_amount NUMERIC(78, 0) NOT NULL,
    launcher_amount NUMERIC(78, 0) NOT NULL,
    frontend TEXT NOT NULL,
    frontend_amount NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS basket_fee_accrual_events_basket_time_idx
    ON basket_fee_accrual_events (basket, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS basket_fee_claim_events (
    id TEXT PRIMARY KEY,
    basket TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    beneficiary TEXT NOT NULL,
    amount NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS basket_fee_claim_events_beneficiary_time_idx
    ON basket_fee_claim_events (beneficiary, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS basket_auctions (
    id TEXT PRIMARY KEY,
    creator TEXT NOT NULL DEFAULT '',
    eth_amount NUMERIC(78, 0) NOT NULL DEFAULT 0,
    spot_quote NUMERIC(78, 0) NOT NULL DEFAULT 0,
    initial_bid NUMERIC(78, 0) NOT NULL DEFAULT 0,
    highest_bid NUMERIC(78, 0) NOT NULL DEFAULT 0,
    highest_bidder TEXT NOT NULL DEFAULT '',
    start_time BIGINT NOT NULL DEFAULT 0,
    end_time BIGINT NOT NULL DEFAULT 0,
    settled_at BIGINT,
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS basket_auction_bid_events (
    id TEXT PRIMARY KEY,
    auction_id TEXT NOT NULL,
    bidder TEXT NOT NULL,
    total_bid NUMERIC(78, 0) NOT NULL,
    is_initial BOOLEAN NOT NULL DEFAULT FALSE,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS basket_auction_bid_events_auction_idx
    ON basket_auction_bid_events (auction_id, block_timestamp, log_index);

CREATE TABLE IF NOT EXISTS basket_auction_results (
    auction_id TEXT PRIMARY KEY,
    winner TEXT NOT NULL,
    eth_amount NUMERIC(78, 0) NOT NULL,
    bid_token_burned NUMERIC(78, 0) NOT NULL,
    settled_at BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE TABLE IF NOT EXISTS basket_auction_account_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    auction_id TEXT,
    account TEXT NOT NULL,
    recipient TEXT,
    amount NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE TABLE IF NOT EXISTS basket_rebalances (
    id TEXT PRIMARY KEY,
    basket TEXT NOT NULL,
    nav_before NUMERIC(78, 0) NOT NULL,
    nav_after NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS basket_rebalances_basket_time_idx
    ON basket_rebalances (basket, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS pump_summary (
    id TEXT PRIMARY KEY,
    token_counts BIGINT NOT NULL DEFAULT 0,
    listed_counts BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS pairs (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL,
    token_index INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    joined_at BIGINT,
    entity_index BIGINT NOT NULL DEFAULT 0,
    ipshare_index BIGINT NOT NULL DEFAULT 0,
    share_supply NUMERIC(78, 0) NOT NULL DEFAULT 0,
    ipshare_create_block BIGINT NOT NULL DEFAULT 0,
    fee_amount NUMERIC(78, 0) NOT NULL DEFAULT 0,
    capture_count BIGINT NOT NULL DEFAULT 0,
    total_captured NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_staked NUMERIC(78, 0) NOT NULL DEFAULT 0,
    holders_count BIGINT NOT NULL DEFAULT 0,
    holdings_count BIGINT NOT NULL DEFAULT 0,
    stakers_count BIGINT NOT NULL DEFAULT 0,
    staked_count BIGINT NOT NULL DEFAULT 0,
    walnut_operation_count BIGINT NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS accounts_ipshare_index_idx
    ON accounts (ipshare_index) WHERE ipshare_index > 0;
CREATE INDEX IF NOT EXISTS accounts_entity_index_idx ON accounts (entity_index);

CREATE TABLE IF NOT EXISTS ipshare_summary (
    id TEXT PRIMARY KEY,
    users_count BIGINT NOT NULL DEFAULT 0,
    total_protocol_fee NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_create_fee NUMERIC(78, 0) NOT NULL DEFAULT 0,
    buy_count BIGINT NOT NULL DEFAULT 0,
    sell_count BIGINT NOT NULL DEFAULT 0,
    total_value_capture NUMERIC(78, 0) NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ipshare_holders (
    id TEXT PRIMARY KEY,
    holder TEXT NOT NULL DEFAULT '',
    subject TEXT NOT NULL DEFAULT '',
    shares_owned NUMERIC(78, 0) NOT NULL DEFAULT 0,
    created_at BIGINT,
    UNIQUE (holder, subject)
);
CREATE INDEX IF NOT EXISTS ipshare_holders_holder_idx ON ipshare_holders (holder);
CREATE INDEX IF NOT EXISTS ipshare_holders_subject_idx ON ipshare_holders (subject);

CREATE TABLE IF NOT EXISTS ipshare_stakers (
    id TEXT PRIMARY KEY,
    staker TEXT NOT NULL DEFAULT '',
    subject TEXT NOT NULL DEFAULT '',
    staked_amount NUMERIC(78, 0) NOT NULL DEFAULT 0,
    created_at BIGINT,
    UNIQUE (staker, subject)
);
CREATE INDEX IF NOT EXISTS ipshare_stakers_staker_idx ON ipshare_stakers (staker);
CREATE INDEX IF NOT EXISTS ipshare_stakers_subject_idx ON ipshare_stakers (subject);

CREATE TABLE IF NOT EXISTS ipshare_trade_events (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE,
    trader TEXT NOT NULL, subject TEXT NOT NULL, is_buy BOOLEAN NOT NULL,
    share_amount NUMERIC(78, 0) NOT NULL, eth_amount NUMERIC(78, 0) NOT NULL,
    protocol_eth_amount NUMERIC(78, 0) NOT NULL,
    subject_eth_amount NUMERIC(78, 0) NOT NULL, supply NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL, block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL, log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE TABLE IF NOT EXISTS ipshare_value_capture_events (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE,
    subject TEXT NOT NULL, investor TEXT NOT NULL, amount NUMERIC(78, 0) NOT NULL,
    block_number BIGINT NOT NULL, block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL, log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);

CREATE TABLE IF NOT EXISTS ipshare_stake_events (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE,
    staker TEXT NOT NULL, subject TEXT NOT NULL, is_stake BOOLEAN NOT NULL,
    share_amount NUMERIC(78, 0) NOT NULL, block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL, transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL, UNIQUE (transaction_hash, log_index)
);

CREATE TABLE IF NOT EXISTS walnut_summary (
    id TEXT PRIMARY KEY,
    tvl NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_communities BIGINT NOT NULL DEFAULT 0,
    total_users BIGINT NOT NULL DEFAULT 0,
    total_pools BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_communities (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    created_at BIGINT NOT NULL DEFAULT 0, owner TEXT NOT NULL DEFAULT '',
    dao_fund TEXT NOT NULL DEFAULT '', fee_ratio INTEGER NOT NULL DEFAULT 0,
    c_token TEXT NOT NULL DEFAULT '', treasury TEXT NOT NULL DEFAULT '',
    distributed_c_token NUMERIC(78,0) NOT NULL DEFAULT 0,
    revenue NUMERIC(78,0) NOT NULL DEFAULT 0,
    retained_revenue NUMERIC(78,0) NOT NULL DEFAULT 0,
    users_count BIGINT NOT NULL DEFAULT 0, pools_count BIGINT NOT NULL DEFAULT 0,
    active_pool_count BIGINT NOT NULL DEFAULT 0,
    operation_count BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_pools (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    pool_index BIGINT NOT NULL DEFAULT 0, created_at BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '',
    pool_factory TEXT NOT NULL DEFAULT '', community TEXT NOT NULL DEFAULT '',
    ratio INTEGER NOT NULL DEFAULT 0, asset TEXT NOT NULL DEFAULT '',
    chain_id INTEGER,
    total_amount NUMERIC(78,0) NOT NULL DEFAULT 0, tvl NUMERIC(78,0),
    stakers_count BIGINT NOT NULL DEFAULT 0, lock_duration NUMERIC(78,0),
    pool_type TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS walnut_pools_community_idx ON walnut_pools (community, entity_index);

CREATE TABLE IF NOT EXISTS walnut_account_communities (
    id TEXT PRIMARY KEY, account TEXT NOT NULL, community TEXT NOT NULL,
    created_at BIGINT NOT NULL, UNIQUE(account, community)
);
CREATE INDEX IF NOT EXISTS walnut_account_communities_community_idx
    ON walnut_account_communities (community, account);

CREATE TABLE IF NOT EXISTS walnut_account_pools (
    id TEXT PRIMARY KEY, account TEXT NOT NULL, pool TEXT NOT NULL,
    created_at BIGINT NOT NULL, UNIQUE(account, pool)
);
CREATE INDEX IF NOT EXISTS walnut_account_pools_pool_idx
    ON walnut_account_pools (pool, account);

CREATE TABLE IF NOT EXISTS walnut_pool_stakers (
    id TEXT PRIMARY KEY, account TEXT NOT NULL, pool TEXT NOT NULL,
    created_at BIGINT NOT NULL, UNIQUE(account, pool)
);
CREATE INDEX IF NOT EXISTS walnut_pool_stakers_pool_idx
    ON walnut_pool_stakers (pool, account);

CREATE TABLE IF NOT EXISTS walnut_operations (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE,
    operation_type TEXT NOT NULL, community TEXT NOT NULL,
    pool_factory TEXT, pool TEXT, account TEXT NOT NULL, chain_id INTEGER,
    asset TEXT, amount NUMERIC(78,0), social_order_id NUMERIC(78,0),
    social_harvested BOOLEAN, block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL, transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL, UNIQUE(transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS walnut_operations_account_idx ON walnut_operations (account, entity_index);
CREATE INDEX IF NOT EXISTS walnut_operations_community_idx ON walnut_operations (community, entity_index);
