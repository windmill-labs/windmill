SET memory_limit='1GB';
SELECT count(*) AS n, sum(len(l)) AS total FROM (SELECT i % 4 AS k, list(md5(i::VARCHAR)) AS l FROM range(60000000) t(i) GROUP BY k);
