# RH production deployment

The production data paths are:

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
The Basket-only range is `16303863:`. Both stop blocks remain empty. Each output
module owns a separate PostgreSQL cursor. Use the mismatch policy `ignore` only
for the reviewed one-time legacy cursor migration, then restore `error` before
normal operation.

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
