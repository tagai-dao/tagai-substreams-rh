BEGIN;

-- Factory creation events reuse the event amount field for pool metadata such
-- as ERC20 locking duration. Legacy Graph entities intentionally keep the
-- ADMINADDPOOL operation amount NULL.
UPDATE walnut_operations
SET amount = NULL
WHERE operation_type = 'ADMINADDPOOL'
  AND amount IS NOT NULL;

COMMIT;
