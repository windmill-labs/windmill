pub fn test_duckdb_bug() {
    let conn = duckdb::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "ATTACH 'ducklake:test.db' AS dl; USE dl; CREATE TABLE test (id INTEGER NOT NULL);",
    )
    .unwrap();
    let mut stmt = conn.prepare("INSERT INTO test VALUES (NULL);").unwrap();
    let mut _rows = stmt.query([]).unwrap();
}
