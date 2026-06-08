// Pure helpers for the util-group panel math. Kept separate from
// render_report.ts so they can be unit-tested without dragging in d3/jsdom
// (drawGraphMulti pulls a ~10MB dep tree, far too heavy for a formula test).

// Oversaturation percent: how many more runnable processes than CPUs, as a
// percentage of ncpu. >0 means there's queued work waiting for CPU.
//
//   procs_running = 4, ncpu = 4   →   0%   (exactly utilized)
//   procs_running = 8, ncpu = 4   →   100% (one extra core's worth queued)
//   procs_running = 4, ncpu = 8   →   0%   (clamped — never negative)
//
// Prefers `procs_running` from /proc/stat (runnable only — excludes D-state
// processes waiting on disk/network). Falls back to loadavg-1min when the
// poller didn't capture procs_running (older bench reports). loadavg
// overcounts because it includes D-state, which is why we switched.
export function computeOversatPct(
  ncpu: number,
  opts: { procs_running?: number | null; load1?: number | null },
): number {
  if (!Number.isFinite(ncpu) || ncpu < 1) return 0;
  const runnable = (typeof opts.procs_running === "number" && Number.isFinite(opts.procs_running))
    ? opts.procs_running
    : (typeof opts.load1 === "number" && Number.isFinite(opts.load1) ? opts.load1 : NaN);
  if (!Number.isFinite(runnable)) return 0;
  return Math.max(0, ((runnable - ncpu) / ncpu) * 100);
}
