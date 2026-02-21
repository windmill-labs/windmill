export interface WorktreeInfo {
  branch: string;
  agent: string;
  mux: string;
  path: string;
  dir: string | null;
  status: string;
  elapsed: string;
  title: string;
  profile: string | null;
  agentName: string | null;
  backendPort: number | null;
  frontendPort: number | null;
  backendRunning: boolean;
  frontendRunning: boolean;
}
