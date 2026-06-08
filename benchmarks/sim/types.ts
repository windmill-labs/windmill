// Shared types that don't depend on the existing benchmark harness. Pulling
// these out of workload.ts keeps test modules (report_test.ts, sim_test.ts)
// from transitively importing benchmark_oneoff.ts → lib.ts, which has a
// pre-existing type error unrelated to the sim.

export type WorkloadResult = {
  kind: string;
  jobs: number;
  throughput: number;
  ts: number;
};

// Provisioner output: a handle the rest of the sim can use to run the
// workload + collect metrics. Provisioned by SimProvisioner.provision().
export type NodeHandle = {
  id: string;
  api_host: string;       // base URL the harness pushes at, empty for agent nodes
  worker_token?: string;
};

export type Provisioned = {
  postgres_url: string;          // for the harness's collector connection
  postgres_internal_url: string; // for cross-container references
  nodes: NodeHandle[];
};
