// Per-job parameter distributions for the synthetic `random` script pattern.
//
// Three supported distribution types — picked because they cover the common
// shapes for a job-scheduler workload:
//   uniform     — flat range, predictable for stress runs
//   lognormal   — heavy-tail; matches real durations (lots of small + a few big)
//   categorical — discrete weighted choice (e.g. sleep vs busy mode)
//
// Add more (pareto, normal, etc.) here if a workload needs them.

// Optional clamp on the sampled value. `cap` and `floor` are post-sample —
// any value above `cap` becomes `cap`, below `floor` becomes `floor`. Use
// these to prevent lognormal's long tail from sampling impossible values
// (e.g. 50 GB RAM, 60 s durations).
type ClampOpts = { cap?: number; floor?: number };

export type DistSpec =
  | ({ dist: "uniform"; min: number; max: number } & ClampOpts)
  | ({ dist: "normal"; mean: number; stddev: number } & ClampOpts)
  | ({ dist: "lognormal"; median: number; shape: number } & ClampOpts)
  | { dist: "categorical"; weights: Record<string, number> }
  // Weighted mix of N sub-distributions. With 2 components this is the
  // classic bimodal shape (e.g. 70% fast CRUD jobs + 30% slow ETL).
  // Weights need not sum to 1 — they're relative.
  | { dist: "mixture"; components: { weight: number; spec: DistSpec }[] };

// Static (legacy) workload — one set of distributions for the whole bench.
export type StaticWorkloadConfig = {
  ram_mb: DistSpec;
  duration_ms: DistSpec;
  mode: DistSpec; // expected to be categorical with "sleep" / "busy"
};

// Phased workload — time-bounded blocks, each with its own distributions
// and an active-pusher count. Useful for simulating diurnal patterns or
// step-function load changes (warmup → peak → cooldown).
export type WorkloadPhase = StaticWorkloadConfig & {
  name?: string;
  duration_s: number;
  pushers: number; // max active pushers in this phase (others sleep)
};

export type PhasedWorkloadConfig = {
  phases: WorkloadPhase[];
};

export type WorkloadConfig = StaticWorkloadConfig | PhasedWorkloadConfig;

export function isPhasedConfig(cfg: WorkloadConfig): cfg is PhasedWorkloadConfig {
  return (cfg as PhasedWorkloadConfig).phases !== undefined;
}

// Total bench wall-time covered by a phased config (sum of phase durations).
// Static configs return undefined — caller uses --seconds for their window.
export function totalPhasedDurationS(cfg: WorkloadConfig): number | undefined {
  if (!isPhasedConfig(cfg)) return undefined;
  return cfg.phases.reduce((a, p) => a + p.duration_s, 0);
}

// Find which phase covers a given elapsed time. Past the last phase end,
// returns the last phase (caller decides whether to keep pushing or stop).
export function findActivePhase(
  cfg: PhasedWorkloadConfig,
  elapsed_s: number,
): { phase: WorkloadPhase; index: number; phase_elapsed_s: number } {
  let acc = 0;
  for (let i = 0; i < cfg.phases.length; i++) {
    const p = cfg.phases[i];
    if (elapsed_s < acc + p.duration_s) {
      return { phase: p, index: i, phase_elapsed_s: elapsed_s - acc };
    }
    acc += p.duration_s;
  }
  const last = cfg.phases.length - 1;
  return { phase: cfg.phases[last], index: last, phase_elapsed_s: elapsed_s - acc };
}

export type JobParams = {
  ram_mb: number;
  duration_ms: number;
  mode: "sleep" | "busy";
};

// Box-Muller — one normal sample per call, plenty for per-job use.
function gauss(): number {
  let u1 = Math.random();
  while (u1 === 0) u1 = Math.random();
  const u2 = Math.random();
  return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
}

function clamp(v: number, spec: { cap?: number; floor?: number }): number {
  if (spec.cap !== undefined && v > spec.cap) v = spec.cap;
  if (spec.floor !== undefined && v < spec.floor) v = spec.floor;
  return v;
}

function sample(spec: DistSpec): number | string {
  switch (spec.dist) {
    case "uniform":
      return clamp(spec.min + Math.random() * (spec.max - spec.min), spec);
    case "normal":
      return clamp(spec.mean + spec.stddev * gauss(), spec);
    case "lognormal":
      // median = e^μ, shape = σ.  log(median) gives μ, then add σ·gauss().
      return clamp(Math.exp(Math.log(spec.median) + spec.shape * gauss()), spec);
    case "categorical": {
      const total = Object.values(spec.weights).reduce((a, b) => a + b, 0);
      let r = Math.random() * total;
      for (const [k, w] of Object.entries(spec.weights)) {
        r -= w;
        if (r <= 0) return k;
      }
      // fallthrough on rounding — return the last key
      const keys = Object.keys(spec.weights);
      return keys[keys.length - 1];
    }
    case "mixture": {
      const total = spec.components.reduce((a, c) => a + c.weight, 0);
      let r = Math.random() * total;
      for (const c of spec.components) {
        r -= c.weight;
        if (r <= 0) return sample(c.spec);
      }
      // fallthrough on rounding — sample the last component
      return sample(spec.components[spec.components.length - 1].spec);
    }
  }
}

function asNumber(v: number | string, ctx: string): number {
  if (typeof v !== "number" || !Number.isFinite(v)) {
    throw new Error(`workload[${ctx}]: expected numeric distribution, got ${typeof v}`);
  }
  return v;
}

export function sampleJobParams(
  cfg: WorkloadConfig,
  opts: { elapsed_s?: number } = {},
): JobParams {
  const spec: StaticWorkloadConfig = isPhasedConfig(cfg)
    ? findActivePhase(cfg, opts.elapsed_s ?? 0).phase
    : cfg;
  const mode = sample(spec.mode);
  if (mode !== "sleep" && mode !== "busy") {
    throw new Error(`workload[mode]: expected "sleep" or "busy", got ${JSON.stringify(mode)}`);
  }
  return {
    ram_mb: Math.max(0, Math.round(asNumber(sample(spec.ram_mb), "ram_mb"))),
    duration_ms: Math.max(0, Math.round(asNumber(sample(spec.duration_ms), "duration_ms"))),
    mode,
  };
}

// Whether pusher index `i` should be sending jobs at this moment, given the
// phased config's per-phase `pushers` cap. Static configs: always active.
export function isPusherActive(
  cfg: WorkloadConfig,
  workerIndex: number,
  elapsed_s: number,
): boolean {
  if (!isPhasedConfig(cfg)) return true;
  return workerIndex < findActivePhase(cfg, elapsed_s).phase.pushers;
}

function validatePhase(p: unknown, ctx: string): void {
  const obj = p as Record<string, unknown>;
  for (const k of ["ram_mb", "duration_ms", "mode"] as const) {
    const v = obj[k] as { dist?: unknown } | undefined;
    if (!v || typeof v.dist !== "string") {
      throw new Error(`workload ${ctx} missing or malformed field "${k}"`);
    }
  }
  if (typeof obj.duration_s !== "number" || obj.duration_s <= 0) {
    throw new Error(`workload ${ctx}: "duration_s" must be a positive number`);
  }
  if (typeof obj.pushers !== "number" || obj.pushers < 0) {
    throw new Error(`workload ${ctx}: "pushers" must be a non-negative number`);
  }
}

export async function loadWorkloadConfig(path: string): Promise<WorkloadConfig> {
  const raw = await Deno.readTextFile(path);
  const parsed = JSON.parse(raw);
  if (Array.isArray(parsed.phases)) {
    if (parsed.phases.length === 0) {
      throw new Error("workload config: phases array is empty");
    }
    parsed.phases.forEach((p: unknown, i: number) => validatePhase(p, `phase[${i}]`));
    return parsed as PhasedWorkloadConfig;
  }
  for (const k of ["ram_mb", "duration_ms", "mode"] as const) {
    if (!parsed[k] || typeof parsed[k].dist !== "string") {
      throw new Error(`workload config missing or malformed field "${k}"`);
    }
  }
  return parsed as WorkloadConfig;
}
