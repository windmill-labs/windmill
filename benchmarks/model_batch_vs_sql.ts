/**
 * Model: Batch Pull vs Direct SQL throughput
 *
 * Calibrated from real benchmarks (3 native workers = 24 subworkers, local PG):
 *   - nativets (fast):   batch 291 j/s, SQL 253 j/s at N=24; batch 108, SQL 88 at N=8
 *   - nativets_sleep:    both ~43.8 j/s at N=24 (bottlenecked by 500ms avg exec time)
 *
 * Per-worker job time model:
 *   T_pw = T_base + T_exec + T_contention(N)
 *   throughput = N / T_pw
 *
 * Batch: T_contention grows linearly with N (HTTP server load)
 *   T_pw_batch(N) = BASE_BATCH + T_exec + SCALE_BATCH × N
 *
 * SQL: T_contention grows quadratically with N (SKIP LOCKED scanning past locked rows)
 *   T_pw_sql(N) = BASE_SQL + T_exec + SCALE_SQL × N²
 *
 * Parameters fitted from 2 data points each (N=8, N=24):
 *   Batch: BASE=69.9ms, SCALE=0.525ms/worker
 *   SQL:   BASE=90.4ms, SCALE=0.0078ms/worker²
 *   (SQL quadratic overtakes batch linear around N~40)
 */

// --- Model parameters (fitted from benchmarks) ---

// Batch: per-worker time = BASE + SCALE_LINEAR * N + T_exec
const BASE_BATCH = 69.9; // ms — base overhead (worker loop, HTTP roundtrip, job completion writes)
const SCALE_BATCH = 0.525; // ms per subworker — linear growth from server load

// SQL: per-worker time = BASE + SCALE_QUAD * N² + T_exec
const BASE_SQL = 90.4; // ms — base overhead (worker loop, poll interval wait, job completion writes)
const SCALE_SQL = 0.0078; // ms per subworker² — quadratic growth from SKIP LOCKED contention

// --- Throughput functions ---

function throughputBatch(subworkers: number, execMs: number): number {
  const tPerWorker = BASE_BATCH + execMs + SCALE_BATCH * subworkers;
  return (subworkers / tPerWorker) * 1000; // jobs/s
}

function throughputSql(subworkers: number, execMs: number): number {
  const tPerWorker = BASE_SQL + execMs + SCALE_SQL * subworkers * subworkers;
  return (subworkers / tPerWorker) * 1000; // jobs/s
}

function pct(batch: number, sql: number): string {
  const diff = ((batch - sql) / sql) * 100;
  return `${diff >= 0 ? "+" : ""}${diff.toFixed(0)}%`;
}

// --- Validation against real data ---

console.log("=== Model Validation (vs real benchmarks) ===\n");
console.log(
  "  Setup                | Model Batch | Real Batch | Model SQL | Real SQL",
);
console.log(
  "  ---------------------|-------------|------------|-----------|--------",
);

const cases = [
  { n: 8, exec: 0, label: "1W nativets", realBatch: 108, realSql: 88 },
  { n: 24, exec: 0, label: "3W nativets", realBatch: 291, realSql: 253 },
  {
    n: 24,
    exec: 500,
    label: "3W sleep(500ms)",
    realBatch: 43.8,
    realSql: 43.8,
  },
];

for (const c of cases) {
  const mb = throughputBatch(c.n, c.exec);
  const ms = throughputSql(c.n, c.exec);
  console.log(
    `  ${c.label.padEnd(21)}| ${mb.toFixed(0).padStart(7)} j/s | ${c.realBatch.toFixed(0).padStart(6)} j/s | ${ms.toFixed(0).padStart(5)} j/s | ${c.realSql.toFixed(0).padStart(4)} j/s`,
  );
}

// --- Projections ---

const workerCounts = [1, 2, 3, 5, 8, 10, 15, 20]; // native workers (×8 subworkers each)
const execTimes = [
  { ms: 0, label: "~0ms (identity)" },
  { ms: 5, label: "5ms" },
  { ms: 20, label: "20ms" },
  { ms: 50, label: "50ms" },
  { ms: 200, label: "200ms" },
  { ms: 500, label: "500ms" },
];

console.log("\n\n=== Projected Throughput (jobs/s) ===\n");

for (const exec of execTimes) {
  console.log(`--- Job duration: ${exec.label} ---\n`);
  console.log(
    "  Native workers (subw) |   Batch   |    SQL    |  Advantage  | Batch wins?",
  );
  console.log(
    "  ----------------------|-----------|----------|-------------|------------",
  );

  for (const w of workerCounts) {
    const n = w * 8;
    const b = throughputBatch(n, exec.ms);
    const s = throughputSql(n, exec.ms);
    const advantage = pct(b, s);
    const wins = b > s * 1.05 ? "  YES" : b > s * 1.01 ? "  marginal" : "  no";
    console.log(
      `  ${String(w).padStart(2)}W (${String(n).padStart(3)})    | ${b.toFixed(0).padStart(5)} j/s | ${s.toFixed(0).padStart(5)} j/s | ${advantage.padStart(8)}    | ${wins}`,
    );
  }
  console.log();
}

// --- Crossover analysis ---

console.log("=== Crossover: min workers where batch is >10% faster ===\n");
console.log("  Job duration | Min workers | Subworkers | Batch j/s | SQL j/s");
console.log("  -------------|-------------|------------|-----------|--------");

for (const exec of execTimes) {
  let found = false;
  for (let w = 1; w <= 50; w++) {
    const n = w * 8;
    const b = throughputBatch(n, exec.ms);
    const s = throughputSql(n, exec.ms);
    if (b > s * 1.1) {
      console.log(
        `  ${exec.label.padEnd(13)}| ${String(w).padStart(5)}W      | ${String(n).padStart(5)}      | ${b.toFixed(0).padStart(5)} j/s | ${s.toFixed(0).padStart(5)} j/s`,
      );
      found = true;
      break;
    }
  }
  if (!found) {
    console.log(
      `  ${exec.label.padEnd(13)}| >50W (never significant at this job duration)`,
    );
  }
}

console.log("\n\n=== Key Takeaways ===\n");
console.log(
  "1. For fast jobs (~0ms): batch pull is always faster, advantage grows with scale",
);
console.log("   - 5 native workers (40 subworkers): ~13% faster");
console.log("   - 10 native workers (80 subworkers): ~25% faster");
console.log("   - 20 native workers (160 subworkers): ~88% faster");
console.log(
  "2. For medium jobs (50ms): batch advantage meaningful from ~5 native workers",
);
console.log(
  "3. For slow jobs (500ms+): only matters at 15+ native workers (120+ subworkers)",
);
console.log(
  "   (but still reduces DB load — fewer pull queries, less index scanning)",
);
console.log(
  "4. The SQL quadratic contention (SKIP LOCKED scanning) is the dominant factor",
);
console.log(
  "   — SQL throughput plateaus around 15-20 native workers while batch keeps scaling",
);
