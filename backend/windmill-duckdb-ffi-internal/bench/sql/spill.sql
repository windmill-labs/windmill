SET memory_limit='256MB';
SET threads=4;
SET preserve_insertion_order=false;
SELECT count(*) AS n, max(s) AS mx FROM (SELECT i, md5(i::VARCHAR) || md5((i+1)::VARCHAR) AS s FROM range(15000000) t(i) ORDER BY s);
