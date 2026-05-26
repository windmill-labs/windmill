# pipeline
# on datatable://main/km2_joined
# on $res:f/admins/intelligent_s3
# on volume://shared/inbox
# freshness 1h
# Publish sink: reads the joined Postgres table and writes a final published
# snapshot into the DuckLake catalog. Demonstrates THREE asset kinds via
# annotations alone (datatable, resource via $res:, volume) plus a freshness
# SLA. The resource ref doubles as a real wmill.get_resource call so the
# script is runnable end-to-end.

import wmill


def main():
    pg = wmill.datatable("main")
    rows = pg.query("SELECT category, event_count, avg_value, lake_avg_value FROM km2_joined").fetch()

    # Resource read — same path declared via `// on $res:` above. The asset
    # parser also picks this up from the SDK call (body-inferred form).
    s3_cfg = wmill.get_resource("f/admins/intelligent_s3")

    lake = wmill.ducklake("main")
    lake.query("DROP TABLE IF EXISTS km2_published").execute()
    lake.query(
        "CREATE TABLE km2_published ("
        " category VARCHAR,"
        " event_count INT,"
        " avg_value DOUBLE,"
        " lake_avg_value DOUBLE,"
        " published_at TIMESTAMP"
        ")"
    ).execute()
    for r in rows:
        lake.query(
            "INSERT INTO km2_published VALUES ($1, $2::INTEGER, $3::DOUBLE, $4::DOUBLE, now())",
            r["category"],
            str(r["event_count"]),
            str(r["avg_value"]),
            "0" if r.get("lake_avg_value") is None else str(r["lake_avg_value"]),
        ).execute()

    return {
        "published": len(rows),
        "s3_endpoint": s3_cfg.get("endPoint") if isinstance(s3_cfg, dict) else None,
    }
