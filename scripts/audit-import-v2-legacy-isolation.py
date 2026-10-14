#!/usr/bin/env python3
"""Compare public legacy COPY data with the stopped backup; never restore into a DB."""
import hashlib
import re
import subprocess


def snapshot(sql):
    result, table, rows, header = {}, None, [], b""
    for line in sql.split(b"\n"):
        if table is not None:
            if line == b"\\.":
                if not table.startswith((b"public.import_v2_", b"public.unified_import_v2_")):
                    if table in result:
                        raise ValueError("duplicate table COPY section")
                    digest = hashlib.sha256(header + b"\0" + b"\0".join(sorted(rows))).hexdigest()
                    result[table] = (len(rows), digest)
                table, rows = None, []
            else:
                rows.append(line)
        elif line.startswith(b"COPY public."):
            match = re.fullmatch(rb"COPY (public\..+) \(.+\) FROM stdin;", line)
            if match is None:
                raise ValueError("unsupported COPY header")
            table, header = match[1], line
            # Normalize identifiers only for exclusion, keeping column headers in fingerprints.
            table = table.replace(b'"', b"")
    if table is not None or not result:
        raise ValueError("incomplete or empty legacy dump")
    return result


def main():
    for unit, prop, expected in [
        ("tiptag-unified-incremental.service", "MainPID", "0"),
        ("tiptag-unified-incremental.timer", "ActiveState", "inactive"),
    ]:
        value = subprocess.check_output(["systemctl", "show", unit, "--property=" + prop, "--value"], text=True).strip()
        if value != expected:
            raise SystemExit("STOP: old index must remain stopped")
    container = "tiptag-substreams-postgres"
    backup = "/root/backups/tiptag-rh-import-v2-cutover-85970291-20261012T085902Z.dump"
    with open(backup, "rb") as source:
        old = subprocess.run(["docker", "exec", "-i", container, "pg_restore", "--data-only", "--file=-"], stdin=source, stdout=subprocess.PIPE, check=True).stdout
    current = subprocess.check_output(["docker", "exec", container, "sh", "-c", "exec pg_dump -U \"${POSTGRES_USER:-postgres}\" -d tiptag_rh --data-only --schema=public"])
    before, after = snapshot(old), snapshot(current)
    changed = sorted(table for table in before.keys() | after.keys() if before.get(table) != after.get(table))
    if changed:
        print("FAIL: changed legacy tables:", ", ".join(table.decode() for table in changed))
        raise SystemExit(1)
    digest = hashlib.sha256(repr(sorted(before.items())).encode()).hexdigest()
    print("PASS: legacy tables unchanged:", len(before))
    print("legacy_data_sha256=" + digest)


if __name__ == "__main__":
    main()
