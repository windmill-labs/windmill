# pipeline
# on datatable://main/km2_summary
# Python transform: pulls from the Postgres summary table via wmill.datatable
# and writes a snapshot into a DuckLake catalog via wmill.ducklake. Exercises
# the Python SDK form of both datatable (read) and ducklake (write).

import wmill


def main():
    src = wmill.datatable("main")
    rows = src.query("SELECT category, event_count, avg_value FROM km2_summary").fetch()

    lake = wmill.ducklake("main")
    lake.query("DROP TABLE IF EXISTS km2_lake").execute()
    lake.query(
        "CREATE TABLE km2_lake ("
        " category VARCHAR,"
        " event_count INT,"
        " avg_value DOUBLE,"
        " snapshotted_at TIMESTAMP"
        ")"
    ).execute()
    for r in rows:
        # Same String() → ::cast workaround the bun→ducklake step uses.
        lake.query(
            "INSERT INTO km2_lake VALUES ($1, $2::INTEGER, $3::DOUBLE, now())",
            r["category"],
            str(r["event_count"]),
            str(r["avg_value"]),
        ).execute()
    return {"snapshotted": len(rows)}
