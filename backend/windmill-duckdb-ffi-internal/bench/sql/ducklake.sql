ATTACH 'ducklake://main' AS dl;
CREATE TABLE IF NOT EXISTS dl.bench AS SELECT i AS id, md5(i::VARCHAR) AS s FROM range(500000) t(i);
SELECT count(*) AS n, max(s) AS mx FROM dl.bench;
