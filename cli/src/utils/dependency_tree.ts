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
      const hash = await wmill.storeRawScriptTemp({
        workspace: workspace.workspaceId,
        requestBody: content,
      });
      tree.setContentHash(path, hash);
    }
  }
}

export type ItemType = "script" | "flow" | "app";

interface DependencyNode {
  content: string;
  stalenessHash: string;  // Hash for staleness detection (includes deps, content, metadata)
  contentHash?: string;   // Hash returned from temp storage upload (content only)
  language: ScriptLanguage;
  metadata: string;
  imports: Set<string>;
  importedBy: Set<string>;
  staleReason?: string;
  // Item metadata for generate-metadata command
  itemType: ItemType;
  folder: string;        // Folder path (for flows/apps) or script path (for scripts)
  isRawApp?: boolean;    // Only set for apps
  isDirectlyStale: boolean;  // True if this item's content changed (vs transitively stale)
}

export class DoubleLinkedDependencyTree {
  private nodes: Map<string, DependencyNode> = new Map();

  async addScript(
    path: string,
    content: string,
    language: ScriptLanguage,
    metadata: string,
    imports: string[],
    rawWorkspaceDependencies: Record<string, string>,
    itemType: ItemType,
    folder: string,
    isDirectlyStale: boolean,
    isRawApp?: boolean
  ): Promise<void> {
    const filteredDeps = filterWorkspaceDependencies(rawWorkspaceDependencies, content, language);
    const stalenessHash = await generateScriptHash(filteredDeps, content, metadata);

    if (!this.nodes.has(path)) {
      this.nodes.set(path, {
        content: "", stalenessHash: "", language: "deno", metadata: "",
        imports: new Set(), importedBy: new Set(),
        itemType: "script", folder: "", isDirectlyStale: false,
      });
    }
    const node = this.nodes.get(path)!;
    node.content = content;
    node.stalenessHash = stalenessHash;
    node.language = language;
    node.metadata = metadata;
    node.itemType = itemType;
    node.folder = folder;
    node.isDirectlyStale = isDirectlyStale;
    node.isRawApp = isRawApp;

    for (const importPath of imports) {
      node.imports.add(importPath);

      if (!this.nodes.has(importPath)) {
        this.nodes.set(importPath, {
          content: "", stalenessHash: "", language: "deno", metadata: "",
          imports: new Set(), importedBy: new Set(),
          itemType: "script", folder: "", isDirectlyStale: false,
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

  getStaleReason(path: string): string | undefined {
    return this.nodes.get(path)?.staleReason;
  }

  getItemType(path: string): ItemType | undefined {
    return this.nodes.get(path)?.itemType;
  }

  getFolder(path: string): string | undefined {
    return this.nodes.get(path)?.folder;
  }

  getIsRawApp(path: string): boolean | undefined {
    return this.nodes.get(path)?.isRawApp;
  }

  getIsDirectlyStale(path: string): boolean {
    return this.nodes.get(path)?.isDirectlyStale ?? false;
  }

  /**
   * Mutates the tree by removing all nodes that are not stale.
   * Uses BFS on reverse graph (importedBy) to find all stale scripts.
   * Starts from nodes with isDirectlyStale=true.
   */
  propagateStaleness(): void {
    // Collect directly stale nodes
    const directlyStale = new Set<string>();
    for (const [path, node] of this.nodes.entries()) {
      if (node.isDirectlyStale) {
        directlyStale.add(path);
        node.staleReason = "content changed";
      }
    }

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
          // Set reason for transitively stale scripts
          const importerNode = this.nodes.get(importer);
          if (importerNode) importerNode.staleReason = `depends on ${scriptPath}`;
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
