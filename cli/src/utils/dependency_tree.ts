/**
 * Double-linked dependency tree for tracking script imports and propagating staleness.
 */

import { Workspace } from "../commands/workspace/workspace.ts";
import * as wmill from "../../gen/services.gen.ts";
import { ScriptLanguage } from "./script_common.ts";
import { filterWorkspaceDependencies, generateScriptHash } from "./metadata.ts";

/**
 * Upload all scripts in the tree to temp storage.
 * Populates contentHash on each node from the backend response.
 */
export async function uploadScripts(
  tree: DoubleLinkedDependencyTree,
  workspace: Workspace
): Promise<void> {
  for (const path of tree.allPaths()) {
    const content = tree.getContent(path);
    if (content) {
      const result = await wmill.storeRawScriptTemp({
        workspace: workspace.workspaceId,
        requestBody: { content },
      });
      tree.setContentHash(path, result.hash);
    }
  }
}

interface DependencyNode {
  content: string;
  stalenessHash: string;  // Hash for staleness detection (includes deps, content, metadata)
  contentHash?: string;   // Hash returned from temp storage upload (content only)
  language: ScriptLanguage;
  metadata: string;
  imports: Set<string>;
  importedBy: Set<string>;
}

export class DoubleLinkedDependencyTree {
  private nodes: Map<string, DependencyNode> = new Map();

  async addScript(
    path: string,
    content: string,
    language: ScriptLanguage,
    metadata: string,
    imports: string[],
    rawWorkspaceDependencies: Record<string, string>
  ): Promise<void> {
    const filteredDeps = filterWorkspaceDependencies(rawWorkspaceDependencies, content, language);
    const stalenessHash = await generateScriptHash(filteredDeps, content, metadata);

    if (!this.nodes.has(path)) {
      this.nodes.set(path, {
        content: "", stalenessHash: "", language: "deno", metadata: "",
        imports: new Set(), importedBy: new Set(),
      });
    }
    const node = this.nodes.get(path)!;
    node.content = content;
    node.stalenessHash = stalenessHash;
    node.language = language;
    node.metadata = metadata;

    for (const importPath of imports) {
      node.imports.add(importPath);

      if (!this.nodes.has(importPath)) {
        this.nodes.set(importPath, {
          content: "", stalenessHash: "", language: "deno", metadata: "",
          imports: new Set(), importedBy: new Set(),
        });
      }
      this.nodes.get(importPath)!.importedBy.add(path);
    }
  }

  getContent(path: string): string | undefined {
    return this.nodes.get(path)?.content;
  }

  getStalenessHash(path: string): string | undefined {
    return this.nodes.get(path)?.stalenessHash;
  }

  getContentHash(path: string): string | undefined {
    return this.nodes.get(path)?.contentHash;
  }

  setContentHash(path: string, hash: string): void {
    const node = this.nodes.get(path);
    if (node) {
      node.contentHash = hash;
    }
  }

  getLanguage(path: string): ScriptLanguage | undefined {
    return this.nodes.get(path)?.language;
  }

  getMetadata(path: string): string | undefined {
    return this.nodes.get(path)?.metadata;
  }

  /**
   * Mutates the tree by removing all nodes that are not stale.
   * Uses BFS on reverse graph (importedBy) to find all stale scripts.
   */
  propagateStaleness(directlyStale: Set<string>): void {
    const allStale = new Set(directlyStale);
    const queue = [...directlyStale];
    const visited = new Set<string>();

    while (queue.length > 0) {
      const scriptPath = queue.shift()!;
      if (visited.has(scriptPath)) continue;
      visited.add(scriptPath);

      const node = this.nodes.get(scriptPath);
      if (!node) continue;

      for (const importer of node.importedBy) {
        if (!allStale.has(importer)) {
          allStale.add(importer);
          queue.push(importer);
        }
      }
    }

    for (const path of [...this.nodes.keys()]) {
      if (!allStale.has(path)) {
        this.nodes.delete(path);
      }
    }
  }

  /**
   * Flatten imports for a script - returns path → contentHash for all transitive imports.
   * Must be called after uploadScripts() has populated contentHash values.
   */
  flatten(scriptPath: string): Record<string, string> {
    const result: Record<string, string> = {};
    const queue = [scriptPath];
    const visited = new Set<string>();

    while (queue.length > 0) {
      const current = queue.shift()!;
      if (visited.has(current)) continue;
      visited.add(current);

      const node = this.nodes.get(current);
      if (!node) continue;

      for (const importPath of node.imports) {
        const importNode = this.nodes.get(importPath);
        if (importNode?.contentHash) {
          result[importPath] = importNode.contentHash;
          queue.push(importPath);
        }
      }
    }

    return result;
  }

  allPaths(): IterableIterator<string> {
    return this.nodes.keys();
  }

  has(path: string): boolean {
    return this.nodes.has(path);
  }

  /**
   * Flatten imports for multiple scripts - returns combined path → hash.
   * Useful for getting all temp_script_refs for a flow/app's inline scripts.
   */
  flattenAll(paths: string[]): Record<string, string> {
    const result: Record<string, string> = {};
    for (const path of paths) {
      Object.assign(result, this.flatten(path));
    }
    return result;
  }

  get size(): number {
    return this.nodes.size;
  }
}
