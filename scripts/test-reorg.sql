\set ON_ERROR_STOP on

-- Deterministic PostgreSQL rollback-model acceptance test. It exercises the
-- same I/U/D history records and reverse order documented and used by
-- substreams-sink-sql. Everything is enclosed in a transaction and rolled
-- back, so it is safe to run against a populated sink database.
BEGIN;

CREATE TEMP TABLE reorg_history (
    id BIGSERIAL PRIMARY KEY,
    op CHAR(1) NOT NULL,
    table_name TEXT NOT NULL,
    pk TEXT NOT NULL,
    prev_value TEXT,
    block_num BIGINT NOT NULL
) ON COMMIT DROP;

-- Block 100: insert a row.
INSERT INTO reorg_history (op, table_name, pk, block_num)
VALUES ('I', 'public.pump_summary', '{"id":"reorg-test"}', 100);
INSERT INTO pump_summary (id, token_counts, listed_counts)
VALUES ('reorg-test', 1, 0);

-- Block 101: update it, retaining the complete previous row.
INSERT INTO reorg_history (op, table_name, pk, prev_value, block_num)
SELECT 'U', 'public.pump_summary', '{"id":"reorg-test"}',
       row_to_json(pump_summary)::TEXT, 101
FROM pump_summary WHERE id = 'reorg-test';
UPDATE pump_summary SET token_counts = 2, listed_counts = 1
WHERE id = 'reorg-test';

-- Block 102: delete it, again retaining the previous row.
INSERT INTO reorg_history (op, table_name, pk, prev_value, block_num)
SELECT 'D', 'public.pump_summary', '{"id":"reorg-test"}',
       row_to_json(pump_summary)::TEXT, 102
FROM pump_summary WHERE id = 'reorg-test';
DELETE FROM pump_summary WHERE id = 'reorg-test';

-- Undo to last valid block 100: reverse block 102 then block 101.
INSERT INTO pump_summary
SELECT * FROM json_populate_record(
    NULL::pump_summary,
    (SELECT prev_value::JSON FROM reorg_history
     WHERE op = 'D' AND block_num = 102)
);

UPDATE pump_summary p
SET (id, token_counts, listed_counts) = (
    SELECT id, token_counts, listed_counts
    FROM json_populate_record(
        NULL::pump_summary,
        (SELECT prev_value::JSON FROM reorg_history
         WHERE op = 'U' AND block_num = 101)
    )
)
WHERE p.id = 'reorg-test';

DELETE FROM reorg_history WHERE block_num > 100;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pump_summary
        WHERE id = 'reorg-test' AND token_counts = 1 AND listed_counts = 0
    ) THEN
        RAISE EXCEPTION 'update/delete rollback did not restore block 100 state';
    END IF;
    IF EXISTS (SELECT 1 FROM reorg_history WHERE block_num > 100) THEN
        RAISE EXCEPTION 'fork history rows were not removed';
    END IF;
END
$$;

-- Undo once more to block 99: reverse the original insert.
DELETE FROM pump_summary
WHERE id = ((SELECT pk::JSON ->> 'id' FROM reorg_history
             WHERE op = 'I' AND block_num = 100));
DELETE FROM reorg_history WHERE block_num > 99;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pump_summary WHERE id = 'reorg-test') THEN
        RAISE EXCEPTION 'insert rollback did not remove the fork row';
    END IF;
    IF EXISTS (SELECT 1 FROM reorg_history) THEN
        RAISE EXCEPTION 'rollback history cleanup failed';
    END IF;
END
$$;

SELECT 'PostgreSQL I/U/D reorg rollback model passed' AS result;
ROLLBACK;
