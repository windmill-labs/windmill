/**
 * Double-linked dependency tree for tracking script imports and propagating staleness.
 */

import { Workspace } from "../commands/workspace/workspace.ts";
import * as wmill from "../../gen/services.gen.ts";
import type { ScriptLang } from "../../gen/types.gen.ts";
import { ScriptLanguage } from "./script_common.ts";
import {
  filterWorkspaceDependencies,
  generateScriptHash,
  checkifMetadataUptodate,
  workspaceDependenciesPathToLanguageAndFilename,
  updateMetadataGlobalLock,
} from "./metadata.ts";
import { generateHash } from "./utils.ts";

/**
 * Diff local scripts against deployed versions, upload only those that differ.
 * Only uploaded (mismatched) scripts get contentHash set, so flatten() returns
 * temp_script_refs only for scripts the backend can't resolve from deployed versions.
 */
export async function uploadScripts(
  tree: DoubleLinkedDependencyTree,
  workspace: Workspace
): Promise<void> {
  // Split into scripts vs workspace deps and compute SHA256(content) for each
  const scriptHashes: Record<string, string> = {};
  const workspaceDeps: { path: string; language: ScriptLang; name?: string; hash: string }[] = [];

  for (const path of tree.allPaths()) {
    const content = tree.getContent(path);
    const itemType = tree.getItemType(path);

    if (itemType === "dependencies") {
      // Empty string is valid for workspace deps (means "no deps") — only skip undefined
      if (content === undefined) continue;
      const info = workspaceDependenciesPathToLanguageAndFilename(path);
      if (info) {
        const hash = await generateHash(content);
        workspaceDeps.push({ path, language: info.language as ScriptLang, name: info.name, hash });
      }
    } else if (itemType === "script") {
      if (!content) continue;
      const hash = await generateHash(content);
      scriptHashes[path] = hash;
    }
    // Skip inline_script, flow, app — they don't need temp storage uploads
  }

  if (Object.keys(scriptHashes).length === 0 && workspaceDeps.length === 0) return;

  console.log("[DEBUG uploadScripts] scriptHashes:", JSON.stringify(scriptHashes));
  console.log("[DEBUG uploadScripts] workspaceDeps:", JSON.stringify(workspaceDeps));

  // Single batch query: find which scripts/deps differ from deployed versions
  const mismatched = await wmill.diffRawScriptsWithDeployed({
    workspace: workspace.workspaceId,
    requestBody: {
      scripts: scriptHashes,
      workspace_deps: workspaceDeps,
    },
  });

  console.log("[DEBUG uploadScripts] mismatched:", JSON.stringify(mismatched));

  // Upload only mismatched scripts to temp storage
  for (const path of mismatched) {
    const content = tree.getContent(path);
    const itemType = tree.getItemType(path);

    if (itemType === "dependencies") {
      // Workspace deps don't need temp storage — just mark as mismatched.
      // Empty string is valid (means the dep file was emptied locally).
      if (content !== undefined) {
        tree.setContentHash(path, "mismatched");
      }
    } else if (content) {
      const hash = await wmill.storeRawScriptTemp({
        workspace: workspace.workspaceId,
        requestBody: content,
      });
      console.log(`[DEBUG uploadScripts] stored temp: ${path} -> hash=${hash}`);
      tree.setContentHash(path, hash);
    }
  }
}

export type ItemType = "script" | "inline_script" | "flow" | "app" | "dependencies";

interface DependencyNode {
  content: string;
  stalenessHash: string;  // Hash for staleness detection (includes deps, content, metadata)
  contentHash?: string;   // Hash for temp storage lookup (content only)
  language: ScriptLanguage;
  metadata: string;
  imports: Set<string>;
  importedBy: Set<string>;
  staleReason?: string;
  // Item metadata for generate-metadata command
  itemType: ItemType;
  folder: string;        // Folder path (for flows/apps) or remote path (for scripts)
  originalPath: string;  // Original path passed to handler (with extension for scripts)
  isRawApp?: boolean;    // Only set for apps
  isDirectlyStale: boolean;  // True if this item's content changed (vs transitively stale)
}

export class DoubleLinkedDependencyTree {
  private nodes: Map<string, DependencyNode> = new Map();
  private workspaceDeps: Record<string, string> = {};

  setWorkspaceDeps(deps: Record<string, string>): void {
    this.workspaceDeps = deps;
  }

  async addNode(
    path: string,
    content: string,
    language: ScriptLanguage,
    metadata: string,
    imports: string[],
    itemType: ItemType,
    folder: string,
    originalPath: string,
    isDirectlyStale: boolean,
    isRawApp?: boolean
  ): Promise<void> {
    const hasWorkspaceDeps = itemType === "script" || itemType === "inline_script";
    const filteredDeps = hasWorkspaceDeps
      ? filterWorkspaceDependencies(this.workspaceDeps, content, language)
      : {};
    const stalenessHash = await generateScriptHash({}, content, metadata);

    if (!this.nodes.has(path)) {
      this.nodes.set(path, {
        content: "", stalenessHash: "", language: "deno", metadata: "",
        imports: new Set(), importedBy: new Set(),
        itemType: "script", folder: "", originalPath: "", isDirectlyStale: false,
      });
    }
    const node = this.nodes.get(path)!;
    node.content = content;
    node.stalenessHash = stalenessHash;
    node.language = language;
    node.metadata = metadata;
    node.itemType = itemType;
    node.folder = folder;
    node.originalPath = originalPath;
    node.isDirectlyStale = isDirectlyStale;
    node.isRawApp = isRawApp;

    // Create nodes for referenced workspace deps with content and language.
    const filteredDepsPaths = Object.keys(filteredDeps);
    for (const depsPath of filteredDepsPaths) {
      if (!this.nodes.has(depsPath)) {
        const depsInfo = workspaceDependenciesPathToLanguageAndFilename(depsPath);
        const contentHash = await generateHash(filteredDeps[depsPath] + depsPath);
        const isUpToDate = await checkifMetadataUptodate(depsPath, contentHash, undefined);
        this.nodes.set(depsPath, {
          content: filteredDeps[depsPath],
          stalenessHash: "", language: depsInfo?.language ?? "deno", metadata: "",
          imports: new Set(), importedBy: new Set(),
          itemType: "dependencies", folder: "", originalPath: depsPath,
          isDirectlyStale: !isUpToDate,
        });
      }
    }

    const allImports = [...imports, ...filteredDepsPaths];
    for (const importPath of allImports) {
      node.imports.add(importPath);

      if (!this.nodes.has(importPath)) {
        this.nodes.set(importPath, {
          content: "", stalenessHash: "", language: "deno", metadata: "",
          imports: new Set(), importedBy: new Set(),
          itemType: "script", folder: "", originalPath: "", isDirectlyStale: false,
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

  getOriginalPath(path: string): string | undefined {
    return this.nodes.get(path)?.originalPath;
  }

  getImports(path: string): Set<string> | undefined {
    return this.nodes.get(path)?.imports;
  }

  /**
   * Returns true if this node has been marked stale (directly or transitively).
   */
  isStale(path: string): boolean {
    return this.nodes.get(path)?.staleReason !== undefined;
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

  }

  /**
   * Walks all transitive imports for a node, calling the callback for each.
   * Callback may return true to stop traversing that branch.
   */
  traverseTransitive(scriptPath: string, callback: (importPath: string, node: DependencyNode) => boolean | void): void {
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
        if (importNode) {
          const stop = callback(importPath, importNode);
          if (!stop) {
            queue.push(importPath);
          }
        }
      }
    }
  }

  allPaths(): IterableIterator<string> {
    return this.nodes.keys();
  }

  /**
   * Returns paths of all stale nodes (those with a staleReason).
   */
  *stalePaths(): IterableIterator<string> {
    for (const [path, node] of this.nodes.entries()) {
      if (node.staleReason) {
        yield path;
      }
    }
  }

  has(path: string): boolean {
    return this.nodes.has(path);
  }

  /**
   * Returns workspace deps that were uploaded as mismatched with remote.
   * These need to be passed as raw_workspace_dependencies in job args
   * so the backend uses local content instead of deployed.
   */
  getMismatchedWorkspaceDeps(): Record<string, string> {
    const result: Record<string, string> = {};
    for (const [path, node] of this.nodes.entries()) {
      if (node.itemType === "dependencies" && node.contentHash && node.content !== undefined) {
        result[path] = node.content;
      }
    }
    return result;
  }

  /**
   * Returns path → contentHash for all transitive imports that have been uploaded.
   * Must be called after uploadScripts() has populated contentHash values.
   */
  getTempScriptRefs(scriptPath: string): Record<string, string> {
    const result: Record<string, string> = {};
    this.traverseTransitive(scriptPath, (_path, node) => {
      if (node.contentHash) {
        result[_path] = node.contentHash;
      }
    });
    if (Object.keys(result).length > 0) {
      console.log(`[DEBUG getTempScriptRefs] for ${scriptPath}:`, JSON.stringify(result));
    }
    return result;
  }

  /**
   * Persist workspace dep hashes to wmill-lock.yaml so getRawWorkspaceDependencies
   * considers them up-to-date on the next run.
   */
  async persistDepsHashes(depsPaths: string[]): Promise<void> {
    for (const path of depsPaths) {
      const node = this.nodes.get(path);
      if (node?.itemType === "dependencies" && node.content !== undefined) {
        const hash = await generateHash(node.content + path);
        await updateMetadataGlobalLock(path, hash);
      }
    }
  }

  get size(): number {
    return this.nodes.size;
  }
}
