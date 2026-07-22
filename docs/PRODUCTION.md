# RH production deployment

The production data path is:

`Pinax Substreams -> substreams-sink-sql -> PostgreSQL -> tagai-api-rh`

The developer PostgreSQL container is not part of production. Run the sink and
PostgreSQL on a server, and keep the Pinax token and database credentials out of
Git.

## 1. Install the immutable artifacts

Copy these files to `/opt/tiptag-substreams`:

- `tiptag-substreams-v0.2.0.spkg`
- `bin/substreams-sink-sql` (`v4.13.1`)
- `scripts/run-sink.sh`

Record and verify the exact package identities before starting a production
backfill. Do not copy hashes from a previous deployment because Rust/Substreams
toolchain changes can alter the package even when the source is unchanged:

```bash
sha256sum /opt/tiptag-substreams/tiptag-substreams-v0.2.0.spkg
substreams info /opt/tiptag-substreams/tiptag-substreams-v0.2.0.spkg
```

## 2. Create and initialize PostgreSQL

Create a dedicated database and user, then let the SQL sink install the schema:

```bash
/opt/tiptag-substreams/bin/substreams-sink-sql setup \
  "$DATABASE_URL" \
  /opt/tiptag-substreams/tiptag-substreams-v0.2.0.spkg
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

Keep it mode `0600`. Copy `deploy/tiptag-substreams.env.example` to
`/etc/tiptag-substreams.env`, fill the PostgreSQL DSN, and install the unit:

```bash
sudo cp deploy/tiptag-substreams.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now tiptag-substreams
```

The production range is `6922897:` with an empty stop block. The sink persists
its cursor in PostgreSQL and resumes after a restart.

## 4. Monitor the backfill

```bash
journalctl -u tiptag-substreams -f
psql "$DATABASE_URL" -f scripts/reconcile.sql
psql "$DATABASE_URL" -c 'SELECT id, block_num, block_id FROM cursors;'
```

Because the package uses block filters, `cursors.block_num` is the most recent
block that produced database output. It can legitimately trail the chain head
during an event-free interval. Use the sink progress logs to determine whether
the stream has reached head; do not infer that solely from the cursor row.

Alerts should cover service restarts, authentication failures, PostgreSQL
errors, and a sink progress log that stops advancing for an abnormal period.

## 5. Cut over the RH API

Keep `SUBSTREAMS_READER_ENABLED=0` until the historical backfill has reached
head and reconciliation passes. Then set these values in `tagai-api/.env.rh`:

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
