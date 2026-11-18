// The pipeline editors a session chat opens with open_preview(kind="pipeline"). In production
// PipelineEditorView mounts one per folder and registers its helpers once the canvas is up;
// here the same state and helper factory run without a canvas, so build_pipeline_node and
// edit_pipeline_node stage drafts exactly as they would in a session.
import { AssetService } from "$lib/gen";
import { DATA_ASSET_KINDS } from "../../../../../frontend/src/lib/components/assets/AssetGraph/cascadeRun";
import { createPipelineAiHelpers } from "../../../../../frontend/src/lib/components/assets/AssetGraph/pipelineAiHelpers";
import { PipelineEditorState } from "../../../../../frontend/src/lib/components/assets/AssetGraph/pipelineEditorState.svelte";
import { resolveGraph } from "../../../../../frontend/src/lib/components/assets/AssetGraph/resolveGraph";
import type { AssetGraphResponse } from "../../../../../frontend/src/lib/components/assets/AssetGraph/types";
import type {
  PipelineAIChatHelpers,
  PipelineToolHelpers,
} from "../../../../../frontend/src/lib/components/copilot/chat/pipeline/core";
import { normalizePipelineFolder } from "../../../../../frontend/src/lib/utils/pipelineFolder";
import type { GlobalDraft } from "../../../../core/validators";

interface Editor {
  state: PipelineEditorState;
  helpers: PipelineAIChatHelpers;
}

export interface EvalPipelineEditors {
  /** Opens the folder's editor, as the session's pipeline preview tab mounts one. */
  open(folder: string): Promise<string>;
  folders(): string[];
  toolHelpers: PipelineToolHelpers;
  /** The nodes the editors staged, as the script drafts a deploy of the pipeline writes. */
  drafts(): GlobalDraft[];
}

export function createEvalPipelineEditors(workspace: string): EvalPipelineEditors {
  const editors = new Map<string, Editor>();

  async function open(folder: string): Promise<string> {
    const key = normalizePipelineFolder(folder);
    if (editors.has(key)) return key;
    const state = new PipelineEditorState();
    state.folder = key;
    const base = (await AssetService.getAssetsGraph({
      workspace,
      folder: key,
      assetKinds: DATA_ASSET_KINDS.join(","),
    })) as AssetGraphResponse;
    const helpers = createPipelineAiHelpers({
      getFolder: () => key,
      getWorkspace: () => workspace,
      getResolvedGraph: () =>
        resolveGraph({
          base,
          drafts: state.drafts,
          liveBodyAssets: state.liveBodyAssets,
          liveAnnotations: state.liveAnnotations,
          // Filled from deployed sources by the view's prefetch; a fixture deploys none.
          inferredWritesByPath: new Map(),
          inferredReadsByPath: new Map(),
          annotatedNativeKindsByPath: new Map(),
          triggerDrafts: state.triggerDrafts.values(),
        }),
      getDrafts: () => state.drafts,
      setDrafts: (next) => (state.drafts = next),
      newDraftLocalId: state.newDraftLocalId,
      hasTriggerDrafts: () => state.triggerDrafts.size > 0,
      getTriggerDrafts: () => state.triggerDrafts,
      setTriggerDraft: state.setTriggerDraft,
      onForgetPath: (path) => {
        state.forgetPath(path);
        state.discardTriggerDraftsFor(path);
      },
    });
    editors.set(key, { state, helpers });
    return key;
  }

  return {
    open,
    folders: () => [...editors.keys()],
    toolHelpers: {
      pipelines: () => [...editors.values()].map((e) => e.helpers),
      pipelineFolders: () => [...editors.keys()],
      ensurePipeline: async (folder) => editors.get(normalizePipelineFolder(folder))?.helpers,
    },
    drafts: () =>
      [...editors.values()].flatMap((e) =>
        [...e.state.drafts.values()].map((d) => ({
          type: "script",
          path: d.script.path,
          language: d.script.language,
          summary: d.script.summary,
          value: d.script.content,
          isDraft: true,
        })),
      ),
  };
}
