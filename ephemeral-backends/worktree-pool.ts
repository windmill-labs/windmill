import { promisify } from "util";
import { exec } from "child_process";
import path from "path";

const execAsync = promisify(exec);

export interface WorktreeInfo {
  id: number;
  path: string;
  inUse: boolean;
  currentCommit?: string;
}

export class WorktreePool {
  private worktrees: Map<number, WorktreeInfo> = new Map();
  private baseWorktreePath: string;
  private nextId: number = 0;

  constructor(baseWorktreePath: string = "../windmill-ephemeral-backends") {
    this.baseWorktreePath = path.resolve(baseWorktreePath);
  }

  /**
   * Initialize the pool by discovering existing worktrees
   */
  async initialize(): Promise<void> {
    console.log("🔍 Initializing worktree pool...");

    // Ensure base directory exists
    try {
      await execAsync(`mkdir -p ${this.baseWorktreePath}`);
    } catch (error) {
      // Directory might already exist
    }

    // Discover existing worktrees
    await this.discoverExistingWorktrees();

    console.log(
      `✓ Worktree pool initialized with ${this.worktrees.size} existing worktree(s)`
    );
  }

  /**
   * Discover existing worktrees from git worktree list
   */
  private async discoverExistingWorktrees(): Promise<void> {
    try {
      const { stdout } = await execAsync("git worktree list --porcelain");
      const lines = stdout.split("\n");

      let currentWorktreePath: string | null = null;
      let isMainWorktree = false;

      for (const line of lines) {
        if (line.startsWith("worktree ")) {
          currentWorktreePath = line.substring("worktree ".length);
          isMainWorktree = false;
        } else if (line.startsWith("branch ")) {
          // Main worktree has a branch entry, so this is NOT a detached HEAD worktree
          // We want to skip the main worktree
          isMainWorktree = true;
        } else if (line === "" && currentWorktreePath) {
          // End of worktree entry
          // Check if this worktree is in our base path and not the main worktree
          if (
            currentWorktreePath.startsWith(this.baseWorktreePath) &&
            !isMainWorktree
          ) {
            // Extract the ID from the path (e.g., .../worktree-0 -> 0)
            const match = currentWorktreePath.match(/worktree-(\d+)$/);
            if (match) {
              const id = parseInt(match[1], 10);
              this.worktrees.set(id, {
                id,
                path: currentWorktreePath,
                inUse: false,
                currentCommit: undefined,
              });
              if (id >= this.nextId) {
                this.nextId = id + 1;
              }
              console.log(`  Found existing worktree: ${currentWorktreePath}`);
            }
          }
          currentWorktreePath = null;
          isMainWorktree = false;
        }
      }
    } catch (error) {
      // If git worktree list fails, we'll start with an empty pool
      console.log("  No existing worktrees found");
    }
  }

  /**
   * Acquire a worktree for the given commit hash
   * Returns a free worktree or creates a new one if needed
   */
  async acquire(commitHash: string): Promise<WorktreeInfo> {
    console.log(`\n📂 Acquiring worktree for commit ${commitHash}...`);

    // Try to find a free worktree
    for (const worktree of this.worktrees.values()) {
      if (!worktree.inUse) {
        console.log(`  Using existing worktree: ${worktree.path}`);
        await this.prepareWorktree(worktree, commitHash);
        worktree.inUse = true;
        worktree.currentCommit = commitHash;
        return worktree;
      }
    }

    // No free worktrees, create a new one
    console.log("  No free worktrees available, creating new one...");
    const newWorktree = await this.createWorktree(commitHash);
    newWorktree.inUse = true;
    newWorktree.currentCommit = commitHash;
    this.worktrees.set(newWorktree.id, newWorktree);
    return newWorktree;
  }

  /**
   * Prepare a worktree for use by discarding changes and checking out the commit
   */
  private async prepareWorktree(
    worktree: WorktreeInfo,
    commitHash: string
  ): Promise<void> {
    console.log(`  Preparing worktree at ${worktree.path}...`);

    try {
      // Discard all local changes
      await execAsync("git reset --hard", { cwd: worktree.path });
      await execAsync("git clean -fd", { cwd: worktree.path });

      // Fetch to ensure we have the latest commits
      await execAsync("git fetch origin", { cwd: worktree.path });

      // Checkout the target commit
      await execAsync(`git checkout ${commitHash}`, { cwd: worktree.path });

      console.log(`  ✓ Worktree prepared and checked out to ${commitHash}`);
    } catch (error) {
      throw new Error(
        `Failed to prepare worktree at ${worktree.path}: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }

  /**
   * Create a new worktree for the given commit hash
   */
  private async createWorktree(commitHash: string): Promise<WorktreeInfo> {
    const id = this.nextId++;
    const worktreePath = path.join(this.baseWorktreePath, `worktree-${id}`);

    console.log(`  Creating new worktree at ${worktreePath}...`);

    try {
      // First, fetch to ensure we have the commit
      await execAsync(`git fetch origin ${commitHash}`);

      // Create the worktree at the specific commit
      // Note: This command must be run from the main repository directory
      await execAsync(`git worktree add ${worktreePath} ${commitHash}`);
      console.log(`  ✓ Worktree created at ${worktreePath}`);

      return {
        id,
        path: worktreePath,
        inUse: false,
        currentCommit: undefined,
      };
    } catch (error) {
      throw new Error(
        `Failed to create worktree at ${worktreePath}: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }

  /**
   * Release a worktree back to the pool
   * The worktree is marked as available but NOT deleted
   */
  async release(worktreeId: number): Promise<void> {
    const worktree = this.worktrees.get(worktreeId);
    if (!worktree) {
      console.warn(`⚠️  Worktree ${worktreeId} not found in pool`);
      return;
    }

    console.log(`\n📂 Releasing worktree ${worktreeId} back to pool...`);

    // Mark as available
    worktree.inUse = false;
    worktree.currentCommit = undefined;

    console.log(`✓ Worktree ${worktreeId} is now available for reuse`);
  }

  /**
   * Get the EE worktree path for a given worktree
   */
  getEEWorktreePath(worktree: WorktreeInfo): string {
    return `${worktree.path}_private`;
  }

  /**
   * Get pool statistics
   */
  getStats(): {
    total: number;
    inUse: number;
    available: number;
  } {
    const total = this.worktrees.size;
    const inUse = Array.from(this.worktrees.values()).filter(
      (w) => w.inUse
    ).length;
    return {
      total,
      inUse,
      available: total - inUse,
    };
  }
}
