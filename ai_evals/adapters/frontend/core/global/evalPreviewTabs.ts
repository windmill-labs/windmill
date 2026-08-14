import {
  setClosePreviewTabsHandler,
  setGetPreviewStatusHandler,
  setOpenPagePreviewHandler,
  setOpenPreviewHandler,
} from "../../../../../frontend/src/lib/components/copilot/chat/global/core";
import type { GlobalActivePreviewContext } from "../../../../../frontend/src/lib/components/copilot/chat/global/core";
import {
  describePreview,
  previewTargetForSessionTarget,
  selectPreviewTabsToClose,
  SessionPreviewTabs,
  whereIs,
} from "../../../../../frontend/src/lib/components/sessions/sessionPreviewTabs.svelte";
import {
  previewLocationContext,
  previewLocationLabel,
  promptSafe,
  resolvePreviewTab,
} from "../../../../../frontend/src/lib/components/sessions/previewRouter";
import type { ArtifactVersionTarget } from "../../../../../frontend/src/lib/components/sessions/previewRouter";
import type { SessionTarget } from "../../../../../frontend/src/lib/components/sessions/sessionState.svelte";

// The side panel a session chat talks to, driven by the production tab model rather than
// by canned tool results — so a case measures what the real open_preview / get_preview_status
// / close_page report about the tabs the reader has. sessionRuntime.svelte.ts (the production
// owner of these handlers) can't run here: it reaches for IndexedDB, stores and live editors.

export interface EvalPreviewTabFixture {
  /** Artifact tab, named by the artifact fixture it shows. `version` pins it, as a reader does. */
  artifact?: { name: string; version?: number };
  /** Workspace page tab, e.g. `{ href: "/runs", label: "Runs" }`. */
  page?: { href: string; label: string };
  /** Editor tab for a workspace item. */
  item?: { kind: SessionTarget["kind"]; path: string };
  /** Tab the reader is looking at. Defaults to the last seeded one. */
  active?: boolean;
}

// Registered once for the whole process, as production does at module load, and dispatched
// by session id: global cases run concurrently, so a per-run registration would have every
// case answering out of whichever run registered last.
const panels = new Map<string, SessionPreviewTabs>();

const NO_SESSION = "No active session; the preview panel is unavailable.";

function panelFor(sessionId: string | undefined): SessionPreviewTabs | undefined {
  return sessionId ? panels.get(sessionId) : undefined;
}

setGetPreviewStatusHandler((sessionId) => {
  const owner = panelFor(sessionId);
  if (!owner) return NO_SESSION;
  return describePreview(owner.tabs, owner.activeId, !!owner.displayedTab);
});

setOpenPreviewHandler(async ({ sessionId, kind, path }) => {
  const owner = panelFor(sessionId);
  if (!owner) return "Error: no active session to open the preview in.";
  const target = previewTargetForSessionTarget(kind, path);
  if (!target) {
    return `Error: ${kind} targets cannot be shown in the preview panel.`;
  }
  // The pipeline branch of the production handler waits on an editor that only exists once
  // a canvas mounts, which never happens here — a pipeline preview reports as any other.
  const result = owner.open(target);
  return result.status === "focused"
    ? `A preview tab is already showing ${kind} "${path}" — focused it.`
    : `Opened ${kind} preview for ${path} in a new tab in the side panel.`;
});

setOpenPagePreviewHandler(({ sessionId, href, label, newTab }) => {
  const owner = panelFor(sessionId);
  if (!owner) return undefined;
  const result = owner.open({ type: "page", href, label }, { forceNewTab: newTab });
  if (result.status === "focused") {
    return `A preview tab is already showing ${label} — focused it.`;
  }
  if (result.status === "retargeted") {
    return `Updated the ${label} preview tab with the requested view.`;
  }
  return `Opened ${label} in a new preview tab in the side panel.`;
});

setClosePreviewTabsHandler(({ sessionId, all, match }) => {
  const owner = panelFor(sessionId);
  if (!owner) return NO_SESSION;
  if (owner.tabs.length === 0) return "The preview panel has no open tabs.";
  const labelFor = (t: (typeof owner.tabs)[number]) =>
    promptSafe(previewLocationLabel(whereIs(t)));
  const doomed = selectPreviewTabsToClose(owner.tabs, { all, match });
  if (doomed.length === 0) {
    return `No open tab matched "${match}". Open tabs: ${owner.tabs.map(labelFor).join(", ")}.`;
  }
  const closedLabels = doomed.map(labelFor);
  for (const t of doomed) owner.close(t.id);
  return `Closed ${closedLabels.length} preview tab${closedLabels.length === 1 ? "" : "s"} (${closedLabels.join(", ")}).`;
});

export interface EvalPreviewPanel {
  /** Mirrors production: a written artifact is shown in the panel. `version` carries the
   * caller's intent for the version picker — `latest` drops a pin the reader had set. */
  openArtifact: (id: string, name: string, version?: ArtifactVersionTarget) => void;
  /** What the user message stamps as ACTIVE PREVIEW, as sessionRuntime's resolver reads it. */
  activePreview: () => GlobalActivePreviewContext | undefined;
  dispose: () => void;
}

export function createEvalPreviewPanel(input: {
  sessionId: string;
  tabs: EvalPreviewTabFixture[];
  /** Artifact ids by name, from the artifact fixture seeding. */
  artifactIds: Map<string, string>;
}): EvalPreviewPanel {
  // Nothing durable to write back to, and no debounce worth waiting on.
  const owner = new SessionPreviewTabs(
    { tabs: [], activeId: "", collapsed: false },
    { persist: () => {} },
    0,
  );
  // Opening a tab makes it the active one, so the fixture's pick can only be applied once
  // every tab is seeded — selecting inside the loop would lose to the next open.
  let requestedActive: string | undefined;
  for (const fixture of input.tabs) {
    const opened = seedTab(owner, fixture, input.artifactIds);
    if (opened && fixture.active) requestedActive = opened;
  }
  if (requestedActive) owner.select(requestedActive);
  // Registered last: seeding throws on a malformed fixture, and this map outlives the run.
  panels.set(input.sessionId, owner);

  return {
    openArtifact: (id, name, version) => {
      owner.open({ type: "artifact", id, name, version });
    },
    activePreview: () => {
      const tab = owner.displayedTab;
      if (!tab) return undefined;
      // Artifact and editor tabs are not iframes: they carry no page location, and an
      // artifact's pinned version reaches the chat only through get_preview_status.
      if (resolvePreviewTab(tab.url).kind !== "iframe") return undefined;
      return previewLocationContext(whereIs(tab));
    },
    dispose: () => {
      panels.delete(input.sessionId);
    },
  };
}

// Seeds one tab through the production open path and returns its id, so a fixture cannot
// describe a tab the panel could not have reached on its own.
function seedTab(
  owner: SessionPreviewTabs,
  fixture: EvalPreviewTabFixture,
  artifactIds: Map<string, string>,
): string | undefined {
  // A tab shows one destination; the branches below would silently keep the first.
  const named = [fixture.artifact, fixture.page, fixture.item].filter(Boolean);
  if (named.length > 1) {
    throw new Error(
      "Preview tab fixture sets more than one of artifact, page and item — a tab shows one of them",
    );
  }
  if (fixture.artifact) {
    const id = artifactIds.get(fixture.artifact.name);
    if (!id) {
      throw new Error(
        `Preview tab fixture references artifact "${fixture.artifact.name}", which no artifact fixture seeds`,
      );
    }
    owner.open({ type: "artifact", id, name: fixture.artifact.name });
    // A pin is the reader's own pick in the version picker, never a side effect of opening.
    if (fixture.artifact.version !== undefined) {
      owner.pinArtifactVersion(id, fixture.artifact.version);
    }
  } else if (fixture.page) {
    owner.open({ type: "page", href: fixture.page.href, label: fixture.page.label });
  } else if (fixture.item) {
    const target = previewTargetForSessionTarget(fixture.item.kind, fixture.item.path);
    if (!target) {
      throw new Error(`Preview tab fixture has an unpreviewable item kind: ${fixture.item.kind}`);
    }
    owner.open(target);
  } else {
    throw new Error("Preview tab fixture must set one of artifact, page or item");
  }
  return owner.activeId;
}
