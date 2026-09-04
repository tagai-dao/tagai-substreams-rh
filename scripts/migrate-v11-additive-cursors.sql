-- PostgreSQL indexes used by tiptag-server's stable RH chain cursors.
-- Safe to run repeatedly against the existing production database.

BEGIN;

CREATE INDEX IF NOT EXISTS tokens_sync_cursor_idx
    ON tokens (creation_block, creation_log_index, id);

CREATE INDEX IF NOT EXISTS token_trade_events_sync_cursor_idx
    ON token_trade_events (block_number, log_index, id);

CREATE INDEX IF NOT EXISTS token_listings_sync_cursor_idx
    ON token_listings (block_number, log_index, token);

CREATE INDEX IF NOT EXISTS imported_token_trade_events_sync_cursor_idx
    ON imported_token_trade_events (block_number, call_index, id);

COMMIT;
