SET temp_directory='s3://nonexistent-bucket/spill';
SET memory_limit='128MB';
SET threads=2;
SELECT count(*) FROM (SELECT md5(i::VARCHAR) s FROM range(10000000) t(i) ORDER BY s);
