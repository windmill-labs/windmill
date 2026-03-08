# Batch Pull Benchmark Results

Date: 2026-03-06
Setup: 1 server + 1 native worker (8 subworkers), standalone mode
Hardware: fedora, 1.5TB disk, ~1GB memory usage
DB: PostgreSQL local, windmill 270 MiB

## nativets — 1000 jobs

|  | Batch Pull | Direct SQL |
|--|-----------|------------|
| Duration | 9.2s | 11.4s |
| **Throughput** | **108 jobs/s** | **88 jobs/s** |
| Improvement | **+23%** | baseline |

### pg_stat_statements

| Query | Batch calls | Batch ms | SQL calls | SQL ms |
|-------|------------|----------|-----------|--------|
| Native pull (FOR UPDATE SKIP LOCKED) | 2,399 (0.02ms avg) | 47 | 2,849 (0.03ms avg) | 87 |
| Default worker pull | 520 (0.04ms avg) | 19 | 445 (0.05ms avg) | 23 |
| DELETE from queue | 1,001 (0.23ms avg) | 231 | 1,001 (0.19ms avg) | 195 |
| INSERT into completed | 1,001 (0.05ms avg) | 51 | 1,001 (0.04ms avg) | 45 |
| INSERT job_logs | 2,003 (0.02ms avg) | 44 | 2,003 (0.02ms avg) | 44 |
| Agent token blacklist | 3,920 (0.00ms avg) | 19 | — | — |
| **Total** | **9,389** | **1,332** | **8,882** | **1,212** |

### pg_stat_database

| Metric | Batch Pull | Direct SQL |
|--------|-----------|------------|
| Transactions committed | 6,484 | 5,868 |
| Blocks read (disk) | 101 | 101 |
| Blocks hit (cache) | 920,257 | 699,032 |
| Tuples returned | 7,784,044 | 8,987,097 |
| Tuples fetched | 1,028,819 | 805,100 |
| Tuples inserted | 6,822 | 6,880 |
| Tuples updated | 3,377 | 3,090 |
| Tuples deleted | 2,883 | 2,654 |

---

## nativets_sleep — 1000 jobs

Each job sleeps 300-700ms (random). Theoretical max with 8 workers: ~16 jobs/s.

|  | Batch Pull | Direct SQL |
|--|-----------|------------|
| Duration | 66.7s | 68.2s |
| **Throughput** | **15.0 jobs/s** | **14.7 jobs/s** |
| Improvement | ~same | baseline |

### pg_stat_statements

| Query | Batch calls | Batch ms | SQL calls | SQL ms |
|-------|------------|----------|-----------|--------|
| Native pull (FOR UPDATE SKIP LOCKED) | 2,399 (0.02ms avg) | 43 | 1,762 (0.04ms avg) | 75 |
| Default worker pull | 1,591 (0.07ms avg) | 113 | 1,392 (0.08ms avg) | 115 |
| DELETE from queue | 1,001 (0.31ms avg) | 308 | 1,001 (0.20ms avg) | 205 |
| INSERT into completed | 1,001 (0.05ms avg) | 46 | 1,001 (0.04ms avg) | 41 |
| INSERT job_logs | 2,003 (0.02ms avg) | 45 | 2,003 (0.02ms avg) | 40 |
| Job runtime ping | 1,490 (0.02ms avg) | 33 | 1,489 (0.02ms avg) | 31 |
| Worker ping (job) | 1,001 (0.03ms avg) | 32 | 1,001 (0.03ms avg) | 30 |
| **Total** | **16,523** | **5,798** | **16,364** | **5,717** |

### pg_stat_database

| Metric | Batch Pull | Direct SQL |
|--------|-----------|------------|
| Transactions committed | 13,638 | 13,388 |
| Blocks read (disk) | 394 | 850 |
| Blocks hit (cache) | 7,033,158 | 4,932,617 |
| Tuples returned | 58,424,571 | 57,204,918 |
| Tuples fetched | 8,776,186 | 6,321,947 |
| Tuples inserted | 7,500 | 7,531 |
| Tuples updated | 6,111 | 6,193 |
| Tuples deleted | 3,006 | 3,196 |

---

## Analysis

**Throughput**: +23% for fast CPU-bound jobs. Negligible difference for I/O-bound jobs.

**Pull queries**: Batch pull does MORE pull queries for nativets_sleep (2,399 vs 1,762). The refiller polls every 50ms even when all workers are busy executing jobs. With direct SQL, workers only poll when idle. This is wasted work — the refiller queries DB and gets empty results while jobs are in-flight.

**Disk I/O**: Batch pull cuts disk reads in half for nativets_sleep (394 vs 850 blocks). Likely because the batch query locks multiple rows in one pass, reducing index traversal.

**Cache hits**: Higher with batch pull (7M vs 4.9M for sleep). More buffer hits from the refiller's repeated empty polls touching the same index pages.

**Tuples fetched**: Higher with batch pull (8.7M vs 6.3M for sleep). Same cause — the refiller's empty polls scan the index.

**At scale**: With 8 subworkers the differences are small. The real benefit is with many native workers where direct SQL SKIP LOCKED contention grows O(N²).
