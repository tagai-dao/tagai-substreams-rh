BEGIN;

-- substreams-sink-sql expresses aggregate updates as INSERT ... ON CONFLICT.
-- PostgreSQL validates NOT NULL columns on the INSERT candidate first, so all
-- immutable Basket creation fields need safe defaults even when the target row
-- already exists and only aggregate columns are being incremented.
ALTER TABLE baskets ALTER COLUMN creator SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN registrar SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN version SET DEFAULT 0;
ALTER TABLE baskets ALTER COLUMN created_at SET DEFAULT 0;
ALTER TABLE baskets ALTER COLUMN creation_block SET DEFAULT 0;
ALTER TABLE baskets ALTER COLUMN creation_block_hash SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN creation_transaction_hash SET DEFAULT '';
ALTER TABLE baskets ALTER COLUMN creation_log_index SET DEFAULT 0;

COMMIT;
