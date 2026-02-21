export interface WorktreeInfo {
  branch: string;
  agent: string;
  mux: string;
  path: string;
  status: string;
  elapsed: string;
  title: string;
  profile: string | null;
  backendPort: number | null;
  frontendPort: number | null;
  backendRunning: boolean;
  frontendRunning: boolean;
}
