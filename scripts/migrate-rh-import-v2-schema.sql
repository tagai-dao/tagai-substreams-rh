\set ON_ERROR_STOP on
BEGIN;
-- Isolated additive domain. No updates/replay of legacy Token/Walnut/V14 aggregates.
CREATE TABLE IF NOT EXISTS import_v2_events (
 id TEXT PRIMARY KEY, kind TEXT NOT NULL, source TEXT NOT NULL, payload TEXT NOT NULL,
 block_number BIGINT NOT NULL, block_hash TEXT NOT NULL, block_timestamp BIGINT NOT NULL,
 transaction_hash TEXT NOT NULL, log_index INTEGER NOT NULL,
 UNIQUE(transaction_hash,log_index)
);
CREATE INDEX IF NOT EXISTS import_v2_events_position ON import_v2_events(block_number,log_index,id);
CREATE TABLE IF NOT EXISTS import_v2_markets (
 token TEXT PRIMARY KEY, community TEXT NOT NULL DEFAULT '', deployer TEXT NOT NULL DEFAULT '',
 social_pool TEXT NOT NULL DEFAULT '', calculator TEXT NOT NULL DEFAULT '', source TEXT NOT NULL DEFAULT '',
 updated_block BIGINT NOT NULL DEFAULT 0, updated_at BIGINT NOT NULL DEFAULT 0,
 updated_log_index INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS import_v2_trade_events (
 id TEXT PRIMARY KEY, entity_index BIGINT NOT NULL UNIQUE, venue TEXT NOT NULL,
 token TEXT NOT NULL, trader TEXT NOT NULL, sellsman TEXT NOT NULL, recipient TEXT NOT NULL,
 is_buy BOOLEAN NOT NULL, token_amount NUMERIC(78,0) NOT NULL,
 native_amount NUMERIC(78,0) NOT NULL, nutbox_token_fee NUMERIC(78,0) NOT NULL,
 block_number BIGINT NOT NULL, block_hash TEXT NOT NULL, block_timestamp BIGINT NOT NULL,
 transaction_hash TEXT NOT NULL, call_index INTEGER NOT NULL,
 UNIQUE(transaction_hash,call_index)
);
CREATE INDEX IF NOT EXISTS import_v2_trade_events_position ON import_v2_trade_events(block_number,call_index,id);
COMMIT;
