// Failed-jobs timeline collector. Called at end-of-bench: pages through
// Windmill's completed-jobs list filtered to `success=false` and within the
// bench's wall window, writes a JSONL timeline that the renderer plots as a
// "Failed jobs over time" panel.
//
// Output (one JSON object per line):
//   {"ts_ms": 1780573...0, "success": false, "job_id": "...", "kind": "..."}
//
// Why we query the API at end-of-bench instead of sampling during: list
// endpoints are slow under heavy bench load and would contend with the worker
// pull queries. One pass at the end is cheaper and complete.

export type FailedJob = {
  ts_ms: number;
  success: boolean;
  job_id?: string;
  kind?: string;
  // Categorised failure mode so the renderer can bin them on a "failures
  // by type" bar chart. Values:
  //   "sigkill_137"      — deno subprocess SIGKILLed (worker heartbeat
  //                        reaper or external SIGKILL — exit_code 137)
  //   "oom_oomkilled"    — container/process explicitly marked OOMKilled
  //   "timeout"          — job timeout from app side
  //   "script_error"     — actual script-level exception
  //   "unknown"          — couldn't categorise
  category?: string;
  exit_code?: number;
  mem_peak_mb?: number;
};

type ApiJob = {
  id: string;
  success: boolean | null;
  // Various candidates depending on EE/OSS endpoint version; we pick whichever
  // is present.
  ended_at?: string;
  completed_at?: string;
  duration_ms?: number;
  created_at?: string;
  job_kind?: string;
  kind?: string;
  mem_peak?: number;
  result?: {
    error?: {
      name?: string;
      message?: string;
      exit_code?: number;
    };
  };
};

function jobTs(j: ApiJob): number {
  for (const k of ["ended_at", "completed_at", "created_at"] as const) {
    const v = j[k];
    if (typeof v === "string") {
      const ms = Date.parse(v);
      if (Number.isFinite(ms)) return ms;
    }
  }
  return 0;
}

export async function collectFailedJobs(
  opts: {
    host: string;
    token: string;
    workspace: string;
    sinceMs: number;
    outPath: string;
  },
): Promise<void> {
  const { host, token, workspace, sinceMs, outPath } = opts;
  // ISO 8601 since-time for the API filter — minus 1s for clock skew between
  // bench host and cluster.
  const sinceIso = new Date(sinceMs - 1000).toISOString();
  const headers = { Authorization: `Bearer ${token}` };
  const out: FailedJob[] = [];

  // Paginate. Windmill's list_completed supports `success=false`,
  // `created_after`, page/per_page. Cap pages at 200 (= 200k jobs) as a
  // sanity guard — a heavy bench shouldn't get near that.
  for (let page = 1; page <= 200; page++) {
    // Correct endpoint is `/api/w/{ws}/jobs/completed/list` — the `jobs_u`
    // routes only have per-id getters, not a list. 404 from the old path is
    // what silently produced "0 failures" in prior reports even when the
    // bench actually had a wave of failed jobs.
    const u = new URL(`${host}/api/w/${workspace}/jobs/completed/list`);
    u.searchParams.set("page", String(page));
    u.searchParams.set("per_page", "1000");
    u.searchParams.set("success", "false");
    u.searchParams.set("created_after", sinceIso);
    const res = await fetch(u, { headers });
    if (!res.ok) {
      console.warn(`[failed-jobs] page ${page} HTTP ${res.status} — stopping`);
      break;
    }
    const arr = (await res.json()) as ApiJob[];
    if (!Array.isArray(arr) || arr.length === 0) break;
    for (const j of arr) {
      const err = j.result?.error;
      const msg = err?.message ?? "";
      const exit = err?.exit_code;
      let category = "unknown";
      if (exit === 137 || /signal:?\s*9|SIGKILL/i.test(msg)) category = "sigkill_137";
      else if (/OOMKilled|out of memory/i.test(msg)) category = "oom_oomkilled";
      else if (/timeout|did not receive .* ping/i.test(msg)) category = "timeout";
      else if (err?.name === "ExecutionErr" && exit !== undefined && exit !== 0) category = "exit_" + exit;
      else if (err) category = "script_error";
      out.push({
        ts_ms: jobTs(j),
        success: !!j.success,
        job_id: j.id,
        kind: j.job_kind ?? j.kind,
        category,
        exit_code: exit,
        mem_peak_mb: j.mem_peak,
      });
    }
    if (arr.length < 1000) break;
  }

  out.sort((a, b) => a.ts_ms - b.ts_ms);
  await Deno.writeTextFile(
    outPath,
    out.map((r) => JSON.stringify(r)).join("\n") + (out.length ? "\n" : ""),
  );
  console.log(`[failed-jobs] ${out.length} failures captured -> ${outPath}`);
}
