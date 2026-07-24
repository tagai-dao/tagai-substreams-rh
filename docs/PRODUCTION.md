# RH production deployment

The original production data paths were:

- `Pinax legacy continuation -> substreams-sink-sql -> PostgreSQL`
- `Pinax Basket-only -> substreams-sink-sql -> PostgreSQL`
- `PostgreSQL -> tagai-api-rh`

The developer PostgreSQL container is not part of production. Run the sink and
PostgreSQL on a server, and keep the Pinax token and database credentials out of
Git.

## 1. Install the immutable artifacts

Keep or copy these files to `/opt/tiptag-substreams`:

- the exact server-built `tiptag-substreams-v0.1.0.spkg` already associated
  with the production cursor
- `tiptag-basket-substreams-v0.1.0.spkg`
- `bin/substreams-sink-sql` (`v4.13.1`)
- `scripts/run-sink.sh`

Derive the legacy continuation package from the exact old SPKG. This disables
only the two retired Basket block filters and does not rebuild the old WASM:

```bash
cargo run --release --example make_legacy_continuation -- \
  /opt/tiptag-substreams/tiptag-substreams-v0.1.0.spkg \
  /opt/tiptag-substreams/tiptag-legacy-continuation-v0.1.0.spkg

sha256sum \
  /opt/tiptag-substreams/tiptag-legacy-continuation-v0.1.0.spkg \
  /opt/tiptag-substreams/tiptag-basket-substreams-v0.1.0.spkg
substreams info /opt/tiptag-substreams/tiptag-legacy-continuation-v0.1.0.spkg db_out
substreams info /opt/tiptag-substreams/tiptag-basket-substreams-v0.1.0.spkg basket_db_out
```

Verify that the non-Basket module hashes in the continuation package equal the
original v0.1.0 hashes. Its `db_out` hash must differ because its Basket inputs
now point to disabled filters.

## 2. Create and initialize PostgreSQL

Create a dedicated database and user, then let the SQL sink install the schema:

```bash
/opt/tiptag-substreams/bin/substreams-sink-sql setup \
  "$DATABASE_URL" \
  /opt/tiptag-substreams/tiptag-basket-substreams-v0.1.0.spkg
```

Do not point a new package with a different `db_out` module hash at an existing
cursor unless its schema and state compatibility have been explicitly proven.
`MODULE_HASH_MISMATCH_POLICY` defaults to `error`. A reviewed migration may set
it to `ignore` only while advancing the existing cursor to the new module hash;
restore `error` immediately afterward.

## 3. Configure authentication and systemd

Run `substreams auth` as the service user, or copy the generated export to:

```text
/opt/tiptag-substreams/.substreams.env
```

Keep it mode `0600`. Install both environment files and units:

```bash
sudo cp deploy/tiptag-substreams.service /etc/systemd/system/
sudo cp deploy/tiptag-basket-substreams.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable tiptag-substreams tiptag-basket-substreams
```

The legacy continuation resumes the existing cursor and has range `6922897:`.
The Basket-only range is `18022342:`. Both stop blocks remain empty. The Basket
service must use `CURSORS_TABLE=basket_cursors` and
`HISTORY_TABLE=basket_substreams_history`; sharing the legacy system tables
would copy a cursor below Basket's module initial block. Use the mismatch policy
`ignore` only for the reviewed one-time legacy cursor migration, then restore
`error` before normal operation. A fresh Basket system table starts directly at
its module initial block with strict mismatch handling.

## 4. Monitor the backfill

```bash
journalctl -u tiptag-substreams -f
journalctl -u tiptag-basket-substreams -f
psql "$DATABASE_URL" -f scripts/reconcile.sql
psql "$DATABASE_URL" -c 'SELECT id, block_num, block_id FROM cursors;'
```

Because both packages use block filters, `cursors.block_num` is the most recent
block that produced database output. It can legitimately trail the chain head
during an event-free interval. Use the sink progress logs to determine whether
the stream has reached head; do not infer that solely from the cursor row.

Alerts should cover service restarts, authentication failures, PostgreSQL
errors, and a sink progress log that stops advancing for an abnormal period.

## 5. Cut over the RH API

Keep `SUBSTREAMS_READER_ENABLED=0` until both streams have reached head and
reconciliation passes. Then set these values in `tagai-api/.env.rh`:

```dotenv
CHAIN_ID=4663
SUBSTREAMS_READER_ENABLED=1
SUBSTREAMS_DATABASE_URL=postgres://tiptag:change-me@127.0.0.1:5432/tiptag_rh
```

Restart only the RH process:

```bash
pm2 restart tagai-api-rh --update-env
```

Token and IPShare reads now use PostgreSQL. FPMM remains on `FPMM_NODE`.

## 6. Roll back the reader

The sink may continue running during an API rollback. Set:

```dotenv
SUBSTREAMS_READER_ENABLED=0
```

and restart `tagai-api-rh`. The API immediately returns to the configured Graph
endpoints without changing PostgreSQL data or its sink cursor.

## 7. Consolidate into one production stream

The long-running RH deployment should use one unified stream after the legacy
and Basket backfills have both reached the same live block. The unified package
is assembled from three artifacts:

1. The exact legacy-continuation SPKG already running in production.
2. The exact Basket-only SPKG already running in production.
3. A freshly built combined template SPKG whose `db_out` knows how to write
   both domains.

Do not rebuild the legacy or Basket upstream modules for this migration. Build
the combined template, then assemble and verify the unified artifact:

```bash
substreams build --manifest substreams.yaml

cargo run --release --example make_unified_package -- \
  /opt/tiptag-substreams/tiptag-legacy-continuation-v0.1.0.spkg \
  /opt/tiptag-substreams/tiptag-basket-substreams-v0.1.0.spkg \
  ./tiptag-substreams-v0.2.0.spkg \
  ./tiptag-unified-substreams-v0.1.0.spkg

./scripts/verify-unified-package.sh \
  /opt/tiptag-substreams/tiptag-legacy-continuation-v0.1.0.spkg \
  /opt/tiptag-substreams/tiptag-basket-substreams-v0.1.0.spkg \
  ./tiptag-unified-substreams-v0.1.0.spkg
```

The verifier must pass before deployment. It proves that every legacy upstream
module still has the production legacy hash and each Basket upstream module
still has the production Basket hash. Only the combined `db_out` is new.

Use dedicated `unified_cursors` and `unified_substreams_history` tables. Stop
both existing services at a common cursor, set unified `START_BLOCK` to that
block plus one, and start the unified service with strict module-hash handling.
Never point the new output hash at either existing cursor table.

After stopping both existing services, make the alignment check a hard gate:

```bash
docker exec -i tiptag-substreams-postgres \
  psql -U tiptag -d tiptag_rh \
  < ./scripts/check-unified-cutover.sql
```

If it fails, do not start the unified service. Bring only the lagging service to
the reported leading block with a bounded run, stop it, and repeat the check.

The old services remain installed but stopped until the unified stream has
passed cursor, event-count, aggregate, and reorg checks. This makes rollback
recoverable without deleting SQL data.

## 8. Test one-minute bounded incremental runs

The incremental unit reuses the unified package, cursor, history table, and
database. It queries the RH public RPC immediately before each run and sets the
exclusive stop block to:

```text
eth_blockNumber - LATEST_LAG_BLOCKS + 1
```

With the default `LATEST_LAG_BLOCKS=100`, every run indexes through
`latest - 100`. The timer waits ten seconds after a run completes before
starting the next one. Since the oneshot service must become inactive before
the delay begins, runs cannot overlap.

Never run the continuous and incremental unified units together. For a
temporary test, stop the continuous unit and start (but do not enable) the
timer:

```bash
systemctl stop tiptag-unified-substreams.service
date --iso-8601=seconds > /root/tiptag-incremental-test-start
systemctl start tiptag-unified-incremental.timer
```

After the test, stop the timer and allow any active oneshot run to finish.
Aggregate the maximum processed-block counter from each session rather than
summing every periodic stats line:

```bash
systemctl stop tiptag-unified-incremental.timer
./scripts/sum-incremental-processed-blocks.sh \
  "$(cat /root/tiptag-incremental-test-start)"
```

Restart the continuous unit after a temporary test. Do not enable the timer as
a permanent replacement until its processed-block usage and data freshness
have been accepted.
