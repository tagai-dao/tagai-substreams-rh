\set ON_ERROR_STOP on

DO $$
DECLARE
    legacy_cursor cursors%ROWTYPE;
    basket_cursor basket_cursors%ROWTYPE;
BEGIN
    SELECT * INTO STRICT legacy_cursor
    FROM cursors
    WHERE id = '8a8d17fda121118ff10e0727bdc00cb68fe8ef5f';

    SELECT * INTO STRICT basket_cursor
    FROM basket_cursors
    WHERE id = 'ec07cf9055952923184fd9e863a1643c88ea0a32';

    IF legacy_cursor.block_num <> basket_cursor.block_num
       OR legacy_cursor.block_id <> basket_cursor.block_id THEN
        RAISE EXCEPTION
            'streams are not aligned: legacy=(%, %), basket=(%, %)',
            legacy_cursor.block_num,
            legacy_cursor.block_id,
            basket_cursor.block_num,
            basket_cursor.block_id;
    END IF;
END
$$;

SELECT
    legacy.block_num AS cutover_block,
    legacy.block_id AS cutover_block_id,
    legacy.block_num + 1 AS unified_start_block
FROM cursors AS legacy
WHERE legacy.id = '8a8d17fda121118ff10e0727bdc00cb68fe8ef5f';
