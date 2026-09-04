CREATE TABLE IF NOT EXISTS pump_token_discoveries (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    creator TEXT NOT NULL,
    symbol TEXT NOT NULL,
    pump TEXT NOT NULL,
    version INTEGER NOT NULL,
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

CREATE TABLE IF NOT EXISTS v11_imported_markets (
    token TEXT PRIMARY KEY,
    community TEXT NOT NULL DEFAULT '',
    deployer TEXT NOT NULL DEFAULT '',
    social_pool TEXT NOT NULL DEFAULT '',
    calculator TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT '',
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS imported_token_trade_events (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE, venue TEXT NOT NULL,
    token TEXT NOT NULL, trader TEXT NOT NULL, sellsman TEXT NOT NULL, recipient TEXT NOT NULL,
    is_buy BOOLEAN NOT NULL, token_amount NUMERIC(78,0) NOT NULL,
    native_amount NUMERIC(78,0) NOT NULL, nutbox_token_fee NUMERIC(78,0) NOT NULL,
    block_number BIGINT NOT NULL, block_hash TEXT NOT NULL, block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL, call_index INTEGER NOT NULL,
    UNIQUE(transaction_hash, call_index)
);
CREATE INDEX IF NOT EXISTS imported_token_trade_events_token_time_idx
    ON imported_token_trade_events (token, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS nutbox_router_price_pools (
    pool_id TEXT PRIMARY KEY,
    token0 TEXT NOT NULL DEFAULT '',
    token1 TEXT NOT NULL DEFAULT '',
    source_type INTEGER NOT NULL DEFAULT 0,
    source_data TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT FALSE,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS nutbox_router_routes (
    id TEXT PRIMARY KEY,
    token0 TEXT NOT NULL DEFAULT '',
    token1 TEXT NOT NULL DEFAULT '',
    route_hash TEXT NOT NULL DEFAULT '',
    pool_ids TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT FALSE,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS nutbox_community_fee_pools (
    pool_id TEXT PRIMARY KEY,
    community TEXT NOT NULL DEFAULT '',
    token TEXT NOT NULL DEFAULT '',
    calculator TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT FALSE,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS v11_protocol_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    contract TEXT NOT NULL,
    pool_id TEXT NOT NULL DEFAULT '',
    token TEXT NOT NULL DEFAULT '',
    token0 TEXT NOT NULL DEFAULT '',
    token1 TEXT NOT NULL DEFAULT '',
    community TEXT NOT NULL DEFAULT '',
    account TEXT NOT NULL DEFAULT '',
    recipient TEXT NOT NULL DEFAULT '',
    calculator TEXT NOT NULL DEFAULT '',
    route_hash TEXT NOT NULL DEFAULT '',
    pool_ids TEXT NOT NULL DEFAULT '',
    source_type INTEGER NOT NULL DEFAULT 0,
    previous_source_type INTEGER NOT NULL DEFAULT 0,
    amount NUMERIC(78, 0) NOT NULL DEFAULT 0,
    secondary_amount NUMERIC(78, 0) NOT NULL DEFAULT 0,
    flag BOOLEAN NOT NULL DEFAULT FALSE,
    data TEXT NOT NULL DEFAULT '',
    ratio0 INTEGER NOT NULL DEFAULT 0,
    ratio1 INTEGER NOT NULL DEFAULT 0,
    ratio2 INTEGER NOT NULL DEFAULT 0,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    UNIQUE (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS v11_protocol_events_type_time_idx
    ON v11_protocol_events (event_type, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS walnut_index_broker_nft_factories (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    platform_fee_bps INTEGER NOT NULL DEFAULT 30,
    default_index_token TEXT NOT NULL DEFAULT '',
    supported_pumps TEXT NOT NULL DEFAULT '[]', supported_nft_templates TEXT NOT NULL DEFAULT '[]',
    basket_swap_router_versions TEXT NOT NULL DEFAULT '[]', basket_swap_routers TEXT NOT NULL DEFAULT '[]',
    reserved_collection_names TEXT NOT NULL DEFAULT '[]', pool_count BIGINT NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0, creation_transaction_hash TEXT NOT NULL DEFAULT '',
    created_at BIGINT NOT NULL DEFAULT 0, updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_index_broker_nft_pools (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    factory TEXT NOT NULL DEFAULT '', community TEXT NOT NULL DEFAULT '', admin TEXT NOT NULL DEFAULT '',
    nft_template TEXT NOT NULL DEFAULT '', nft_template_kind TEXT NOT NULL DEFAULT '',
    community_token TEXT NOT NULL DEFAULT '', index_mining_token TEXT NOT NULL DEFAULT '',
    renderer TEXT NOT NULL DEFAULT '', funds_receiver TEXT NOT NULL DEFAULT '', amm TEXT NOT NULL DEFAULT '',
    index_token TEXT NOT NULL DEFAULT '', index_basket_version INTEGER NOT NULL DEFAULT 0,
    basket_swap_router TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '', symbol TEXT NOT NULL DEFAULT '',
    community_token_price NUMERIC(78,0) NOT NULL DEFAULT 0,
    index_mining_activation_token_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    recommit_price NUMERIC(78,0) NOT NULL DEFAULT 0, native_price NUMERIC(78,0) NOT NULL DEFAULT 0,
    max_supply NUMERIC(78,0) NOT NULL DEFAULT 0, referral_bps INTEGER NOT NULL DEFAULT 0,
    lock_whitelist_slots BOOLEAN NOT NULL DEFAULT FALSE, reroll_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    total_whitelist_allocation NUMERIC(78,0) NOT NULL DEFAULT 0,
    minimum_index_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    level_thresholds TEXT NOT NULL DEFAULT '[]', level_weights TEXT NOT NULL DEFAULT '[]',
    total_supply NUMERIC(78,0) NOT NULL DEFAULT 0, whitelist_minted NUMERIC(78,0) NOT NULL DEFAULT 0,
    paid_minted NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_community_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_active_index_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    index_reward_per_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    queued_index_rewards NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_rewards_injected NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_rewards_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_platform_fee NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_referral_commission NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_community_token_mint_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_native_mint_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_native_refunded NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_holder_fees_harvested NUMERIC(78,0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0, creation_block_hash TEXT NOT NULL DEFAULT '',
    creation_transaction_hash TEXT NOT NULL DEFAULT '', creation_log_index INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0, updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_index_broker_nft_amms (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    factory TEXT NOT NULL DEFAULT '', pool TEXT NOT NULL DEFAULT '', community_token TEXT NOT NULL DEFAULT '',
    pump TEXT NOT NULL DEFAULT '', nutbox_router TEXT NOT NULL DEFAULT '', price_quote_token TEXT NOT NULL DEFAULT '',
    index_token TEXT NOT NULL DEFAULT '', index_basket_version INTEGER NOT NULL DEFAULT 0,
    basket_swap_router TEXT NOT NULL DEFAULT '', price_source_type INTEGER NOT NULL DEFAULT 0,
    price_source_data TEXT NOT NULL DEFAULT '', active BOOLEAN NOT NULL DEFAULT FALSE,
    normal_fee_bps INTEGER NOT NULL DEFAULT 0, specific_fee_bps INTEGER NOT NULL DEFAULT 0,
    tokens_per_nft NUMERIC(78,0) NOT NULL DEFAULT 0, inventory_count BIGINT NOT NULL DEFAULT 0,
    oldest_token_id NUMERIC(78,0) NOT NULL DEFAULT 0, newest_token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    sold_count BIGINT NOT NULL DEFAULT 0, bought_count BIGINT NOT NULL DEFAULT 0,
    total_community_token_volume NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_trading_fee NUMERIC(78,0) NOT NULL DEFAULT 0, total_platform_fee NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_native_fee_received NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_native_refunded NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_native_invested NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_caller_reward NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_settlement_token_out NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_token_purchased NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_holder_fees_converted NUMERIC(78,0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0, creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0, created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0, updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_index_broker_nft_tokens (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    pool TEXT NOT NULL DEFAULT '', token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    owner TEXT NOT NULL DEFAULT '', buyer TEXT NOT NULL DEFAULT '', whitelist_mint BOOLEAN NOT NULL DEFAULT FALSE,
    referrer_token_id NUMERIC(78,0) NOT NULL DEFAULT 0, referral_count BIGINT NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 1, mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    mining_active BOOLEAN NOT NULL DEFAULT FALSE, index_mining_active BOOLEAN NOT NULL DEFAULT FALSE,
    index_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0, index_reward_debt NUMERIC(78,0) NOT NULL DEFAULT 0,
    settled_pending_index_rewards NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_rewards_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    seed NUMERIC(78,0) NOT NULL DEFAULT 0, reveal_block NUMERIC(78,0) NOT NULL DEFAULT 0,
    reveal_round NUMERIC(78,0) NOT NULL DEFAULT 0, reveal_pending BOOLEAN NOT NULL DEFAULT FALSE,
    in_inventory BOOLEAN NOT NULL DEFAULT FALSE,
    inventory_previous_token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    inventory_next_token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    community_token_amount NUMERIC(78,0) NOT NULL DEFAULT 0, native_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0, creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0, created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0, updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_index_broker_nft_accounts (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL DEFAULT 0 UNIQUE,
    pool TEXT NOT NULL DEFAULT '', account TEXT NOT NULL DEFAULT '', nft_count BIGINT NOT NULL DEFAULT 0,
    community_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    active_index_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    minted_count BIGINT NOT NULL DEFAULT 0, bought_count BIGINT NOT NULL DEFAULT 0,
    sold_count BIGINT NOT NULL DEFAULT 0, whitelist_minted BIGINT NOT NULL DEFAULT 0,
    remaining_whitelist_allowance NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_referral_commission NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_index_rewards_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0, updated_at BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS walnut_index_broker_nft_events (
    id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE, event_type TEXT NOT NULL,
    source TEXT NOT NULL, factory TEXT, pool TEXT, amm TEXT,
    token_id NUMERIC(78,0), secondary_token_id NUMERIC(78,0), account TEXT,
    secondary_account TEXT, asset TEXT, amount NUMERIC(78,0), secondary_amount NUMERIC(78,0),
    tertiary_amount NUMERIC(78,0), ratio INTEGER, level INTEGER, previous_level INTEGER,
    flag BOOLEAN, data TEXT, block_number BIGINT NOT NULL, block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL, transaction_hash TEXT NOT NULL, log_index INTEGER NOT NULL,
    UNIQUE(transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS walnut_index_broker_nft_events_pool_idx
    ON walnut_index_broker_nft_events (pool, block_number, log_index);
CREATE INDEX IF NOT EXISTS walnut_index_broker_nft_events_amm_idx
    ON walnut_index_broker_nft_events (amm, block_number, log_index);

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
CREATE INDEX IF NOT EXISTS tokens_sync_cursor_idx
    ON tokens (creation_block, creation_log_index, id);

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
CREATE INDEX IF NOT EXISTS token_trade_events_sync_cursor_idx
    ON token_trade_events (block_number, log_index, id);

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

CREATE INDEX IF NOT EXISTS token_listings_sync_cursor_idx
    ON token_listings (block_number, log_index, token);

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
    sell_mask INTEGER NOT NULL DEFAULT 0,
    buy_mask INTEGER NOT NULL DEFAULT 0,
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

CREATE TABLE IF NOT EXISTS walnut_nft_pools (
    id TEXT PRIMARY KEY,
    community TEXT NOT NULL DEFAULT '', factory TEXT NOT NULL DEFAULT '',
    admin TEXT NOT NULL DEFAULT '',
    renderer TEXT NOT NULL DEFAULT '', funds_receiver TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '', symbol TEXT NOT NULL DEFAULT '',
    current_batch_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_supply NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_platform_fee NUMERIC(78,0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_block_hash TEXT NOT NULL DEFAULT '',
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS walnut_nft_pools_community_idx
    ON walnut_nft_pools (community, creation_block);

CREATE TABLE IF NOT EXISTS walnut_nft_batches (
    id TEXT PRIMARY KEY,
    pool TEXT NOT NULL DEFAULT '', batch_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    payment_asset TEXT NOT NULL DEFAULT '', mint_price NUMERIC(78,0) NOT NULL DEFAULT 0,
    max_supply NUMERIC(78,0) NOT NULL DEFAULT 0, minted NUMERIC(78,0) NOT NULL DEFAULT 0,
    referral_bps INTEGER NOT NULL DEFAULT 0, palette_id INTEGER NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT TRUE, paused BOOLEAN NOT NULL DEFAULT FALSE,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    UNIQUE(pool, batch_id)
);
CREATE INDEX IF NOT EXISTS walnut_nft_batches_pool_idx
    ON walnut_nft_batches (pool, batch_id);

CREATE TABLE IF NOT EXISTS walnut_nfts (
    id TEXT PRIMARY KEY,
    pool TEXT NOT NULL DEFAULT '', token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    owner TEXT NOT NULL DEFAULT '', batch_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    referrer_token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    referral_count NUMERIC(78,0) NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 0, mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    buyer TEXT NOT NULL DEFAULT '', payment_asset TEXT NOT NULL DEFAULT '',
    mint_price NUMERIC(78,0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    UNIQUE(pool, token_id)
);
CREATE INDEX IF NOT EXISTS walnut_nfts_owner_idx ON walnut_nfts (pool, owner, token_id);

CREATE TABLE IF NOT EXISTS walnut_nft_accounts (
    id TEXT PRIMARY KEY,
    pool TEXT NOT NULL DEFAULT '', account TEXT NOT NULL DEFAULT '',
    nft_count BIGINT NOT NULL DEFAULT 0,
    mining_weight NUMERIC(78,0) NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    UNIQUE(pool, account)
);
CREATE INDEX IF NOT EXISTS walnut_nft_accounts_account_idx
    ON walnut_nft_accounts (account, pool);

CREATE TABLE IF NOT EXISTS walnut_nft_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL, pool TEXT NOT NULL,
    token_id NUMERIC(78,0), secondary_token_id NUMERIC(78,0),
    batch_id NUMERIC(78,0), account TEXT, secondary_account TEXT, asset TEXT,
    amount NUMERIC(78,0), secondary_amount NUMERIC(78,0),
    ratio INTEGER, level INTEGER, previous_level INTEGER, flag BOOLEAN,
    block_number BIGINT NOT NULL, block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL, transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL, UNIQUE(transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS walnut_nft_events_pool_idx
    ON walnut_nft_events (pool, block_number, log_index);

CREATE TABLE IF NOT EXISTS walnut_basket_tvl_pools (
    id TEXT PRIMARY KEY,
    community TEXT NOT NULL DEFAULT '', factory TEXT NOT NULL DEFAULT '',
    basket_registry TEXT NOT NULL DEFAULT '', nft_mining_pool TEXT NOT NULL DEFAULT '',
    nft_reward_bps INTEGER NOT NULL DEFAULT 0,
    lock_duration NUMERIC(78,0) NOT NULL DEFAULT 0, name TEXT NOT NULL DEFAULT '',
    basket_count BIGINT NOT NULL DEFAULT 0,
    total_mining_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_block_hash TEXT NOT NULL DEFAULT '',
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS walnut_basket_tvl_pools_community_idx
    ON walnut_basket_tvl_pools (community, creation_block);

CREATE TABLE IF NOT EXISTS walnut_basket_stakes (
    id TEXT PRIMARY KEY,
    parent_pool TEXT NOT NULL DEFAULT '', basket TEXT NOT NULL DEFAULT '',
    child_pool TEXT NOT NULL DEFAULT '', creator TEXT NOT NULL DEFAULT '',
    nft_token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    mining_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    nft_reward_bps INTEGER NOT NULL DEFAULT 0,
    lock_duration NUMERIC(78,0) NOT NULL DEFAULT 0,
    chain_updated_at BIGINT NOT NULL DEFAULT 0,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    UNIQUE(parent_pool, basket)
);
CREATE INDEX IF NOT EXISTS walnut_basket_stakes_parent_idx
    ON walnut_basket_stakes (parent_pool, basket);

CREATE TABLE IF NOT EXISTS walnut_basket_tvl_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL, parent_pool TEXT NOT NULL, basket TEXT NOT NULL,
    child_pool TEXT, creator TEXT NOT NULL,
    nft_token_id NUMERIC(78,0), amount NUMERIC(78,0),
    secondary_amount NUMERIC(78,0), tertiary_amount NUMERIC(78,0),
    ratio INTEGER,
    block_number BIGINT NOT NULL, block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL, transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL, UNIQUE(transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS walnut_basket_tvl_events_parent_idx
    ON walnut_basket_tvl_events (parent_pool, block_number, log_index);

CREATE TABLE IF NOT EXISTS walnut_basket_child_pools (
    id TEXT PRIMARY KEY,
    parent_pool TEXT NOT NULL DEFAULT '', community TEXT NOT NULL DEFAULT '',
    basket TEXT NOT NULL DEFAULT '', creator TEXT NOT NULL DEFAULT '',
    nft_token_id NUMERIC(78,0) NOT NULL DEFAULT 0,
    nft_reward_bps INTEGER NOT NULL DEFAULT 0,
    lock_duration NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_staked_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_rewards_harvested NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_nft_rewards_accrued NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_nft_rewards_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_holder_fees_harvested NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_community_rewards_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    total_holder_fees_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    closed_parent_rewards_harvested BOOLEAN NOT NULL DEFAULT FALSE,
    creation_block BIGINT NOT NULL DEFAULT 0,
    creation_transaction_hash TEXT NOT NULL DEFAULT '',
    creation_log_index INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS walnut_basket_child_pools_parent_idx
    ON walnut_basket_child_pools (parent_pool, basket);

CREATE TABLE IF NOT EXISTS walnut_basket_child_positions (
    id TEXT PRIMARY KEY,
    child_pool TEXT NOT NULL DEFAULT '', parent_pool TEXT NOT NULL DEFAULT '',
    basket TEXT NOT NULL DEFAULT '', account TEXT NOT NULL DEFAULT '',
    staked_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    withdraw_requested_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    redeemed_amount NUMERIC(78,0) NOT NULL DEFAULT 0,
    community_rewards_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    holder_fees_claimed NUMERIC(78,0) NOT NULL DEFAULT 0,
    updated_block BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    UNIQUE(child_pool, account)
);
CREATE INDEX IF NOT EXISTS walnut_basket_child_positions_account_idx
    ON walnut_basket_child_positions (account, child_pool);

CREATE TABLE IF NOT EXISTS walnut_basket_child_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL, child_pool TEXT NOT NULL, parent_pool TEXT NOT NULL,
    basket TEXT NOT NULL, account TEXT,
    nft_token_id NUMERIC(78,0), amount NUMERIC(78,0),
    secondary_amount NUMERIC(78,0), tertiary_amount NUMERIC(78,0),
    block_number BIGINT NOT NULL, block_hash TEXT NOT NULL,
    block_timestamp BIGINT NOT NULL, transaction_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL, UNIQUE(transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS walnut_basket_child_events_pool_idx
    ON walnut_basket_child_events (child_pool, block_number, log_index);
