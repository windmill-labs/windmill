import { z } from 'zod'
import { $ScriptLang } from '$lib/gen/schemas.gen'
import type { ScriptLang } from '$lib/gen'
import { createToolDef, executeTestRun, findAndReplace, type Tool } from '../shared'
import type { PipelineOutputKind } from '$lib/components/assets/AssetGraph/pipelineTemplates'

// ============================================================================
// Pipeline AI chat tools.
//
// These tools extend the GLOBAL chat mode when the user is on a /pipeline/<folder>
// editor (the page registers `PipelineAIChatHelpers` on the AIChatManager). They
// let the model read the live pipeline graph and BUILD/EDIT pipeline nodes
// (scripts annotated with `// pipeline`). Mutations don't deploy: they stage an
// AI-pending DRAFT on the canvas — rendered as a distinct, dashed node — that the
// user reviews and Accepts (keep) or Rejects (discard), mirroring the flow
// editor's diff/approval loop. This is the "ask permission to build a node"
// surface: the diff IS the staged draft, the approval is Accept/Reject.
//
// The pipeline tools are added on top of the full global tool set, so docs
// search, datatable SQL, and workspace-item tools are already available
// alongside them — this file only carries the pipeline-graph-specific surface.
// ============================================================================

/** Compact, model-facing summary of one node in the pipeline graph. */
export type PipelineNodeSummary = {
	path: string
	language?: ScriptLang
	/** Has a local edit not yet deployed (covers both AI-pending and manual drafts). */
	unsaved: boolean
	/** Staged by the AI this turn and awaiting the user's Accept/Reject. */
	aiPending: boolean
	summary?: string
	/** Asset URIs this node writes (its outputs). */
	writes: string[]
	/** Asset URIs this node reads (its inputs). */
	reads: string[]
	/** Declared `// on <trigger>` execution-DAG bindings (asset URIs or native kinds). */
	triggers: string[]
}

/** Compact, model-facing snapshot of the whole pipeline graph. */
export type PipelineContext = {
	folder: string
	mode: 'view' | 'edit'
	nodes: PipelineNodeSummary[]
	/** All storage assets referenced by the graph, as URIs. */
	assets: string[]
	/** Number of AI-staged proposals currently awaiting review. */
	pendingProposals: number
}

/**
 * Bridge the pipeline page registers on the AIChatManager. Reads expose the live
 * graph; writes stage AI-pending drafts (never deploy) and feed the Accept/Reject
 * review loop. Kept intentionally small — the page owns the draft Map, snapshots,
 * and canvas rendering.
 */
export interface PipelineAIChatHelpers {
	getPipelineContext: () => PipelineContext
	/** Read a node's source (the in-flight draft body if one exists, else deployed). */
	getNodeBody: (path: string) => Promise<{ language: ScriptLang; content: string } | undefined>
	/** Stage a brand-new pipeline node as an AI-pending draft. */
	proposeNode: (input: {
		path: string
		language: ScriptLang
		content: string
		outputKind?: PipelineOutputKind
	}) => Promise<{ path: string }>
	/** Replace an existing node's body, staging it as an AI-pending draft. */
	editNode: (path: string, content: string) => Promise<void>
	/** Drop an AI-pending node from the staged set. Only AI-pending nodes are removable here. */
	removeProposedNode: (path: string) => Promise<void>
	hasPendingProposals: () => boolean
	acceptAllProposals: () => void
	rejectAllProposals: () => void
	/** Preview-run a node (draft body preferred). Returns the started job id. */
	testNode: (path: string, args?: Record<string, any>) => Promise<string | undefined>
}

/** Helper bag the pipeline tools receive from the manager in global mode. */
export type PipelineToolHelpers = { pipeline?: PipelineAIChatHelpers }

function requirePipeline(helpers: PipelineToolHelpers): PipelineAIChatHelpers {
	if (!helpers?.pipeline) {
		throw new Error(
			'No pipeline editor is open. Pipeline tools only work on a /pipeline/<folder> page in edit mode.'
		)
	}
	return helpers.pipeline
}

const scriptLangSchema = z.enum($ScriptLang.enum)

const outputKindSchema = z
	.enum(['none', 'datatable', 'ducklake', 'materialize', 's3_parquet', 's3_object'])
	.describe(
		'Kind of output asset this node materializes, used to seed the output edge on the canvas before the body is parsed: "materialize"/"ducklake" → a DuckLake table, "datatable" → a Postgres data table, "s3_parquet"/"s3_object" → an S3 file, "none" → side-effect only. Defaults to none.'
	)

// ----------------------------------------------------------------------------
// Read tools
// ----------------------------------------------------------------------------

const getPipelineGraphSchema = z.object({})

const getPipelineGraphToolDef = createToolDef(
	getPipelineGraphSchema,
	'get_pipeline_graph',
	"Read the live pipeline graph for the open /pipeline/<folder> editor: its nodes (scripts), each node's language, asset reads/writes, declared triggers, and whether it is unsaved or an AI-pending proposal. Call this before building or editing nodes so you reuse existing assets/paths and understand the current DAG."
)

const readPipelineNodeSchema = z.object({
	path: z.string().describe('Workspace path of the pipeline node (script) to read.')
})

const readPipelineNodeToolDef = createToolDef(
	readPipelineNodeSchema,
	'read_pipeline_node',
	'Read the full source of one pipeline node (its in-flight draft body if it has unsaved edits, otherwise the deployed body). Use before edit_pipeline_node so edits target the exact current text.'
)

// ----------------------------------------------------------------------------
// Mutation tools (stage AI-pending drafts → diff/approval)
// ----------------------------------------------------------------------------

const buildPipelineNodeSchema = z.object({
	path: z
		.string()
		.describe(
			"Workspace path for the new node, e.g. f/<folder>/<name>. Use the open pipeline's folder. Must not collide with an existing node."
		),
	language: scriptLangSchema.describe(
		'Script language. SQL-shaped data work uses duckdb (DuckLake/S3) or postgresql (data tables); bun/python3 for general transforms.'
	),
	content: z
		.string()
		.describe(
			'Full script source. Start it with the `// pipeline` annotation to mark it a pipeline member, declare inputs with `// on <asset-uri|schedule|webhook|...>`, and write outputs via the wmill SDK / SQL so the lineage edges are inferred. Read existing node bodies first to match conventions.'
		),
	output_kind: outputKindSchema.optional()
})

const buildPipelineNodeToolDef = createToolDef(
	buildPipelineNodeSchema,
	'build_pipeline_node',
	'Build a NEW pipeline node and stage it as an AI-pending draft on the canvas for the user to review (Accept keeps it, Reject discards it). Does NOT deploy. The node shows up immediately as a dashed, AI-highlighted node wired by its parsed asset reads/writes. Prefer this over editing for new scripts.',
	{ strict: false }
)

const editPipelineNodeSchema = z.object({
	path: z.string().describe('Workspace path of the node to edit.'),
	old_string: z.string().min(1).describe("Exact text to find in the node's current source."),
	new_string: z.string().describe('Replacement text.'),
	replace_all: z
		.boolean()
		.optional()
		.default(false)
		.describe(
			'When true, replace every exact match. When false, old_string must match exactly once.'
		)
})

const editPipelineNodeToolDef = createToolDef(
	editPipelineNodeSchema,
	'edit_pipeline_node',
	'Edit an existing pipeline node by exact find/replace, staging the result as an AI-pending draft for review (Accept/Reject). Does NOT deploy. Call read_pipeline_node first to get the exact current text.',
	{ strict: false }
)

const removePipelineNodeSchema = z.object({
	path: z.string().describe('Workspace path of the AI-pending node to drop from the staged set.')
})

const removePipelineNodeToolDef = createToolDef(
	removePipelineNodeSchema,
	'remove_pipeline_node',
	'Remove an AI-pending node you previously staged this turn (undo a build_pipeline_node). Only AI-pending proposals can be removed — to delete a deployed or manually-drafted node, ask the user to do it on the canvas.'
)

const testPipelineNodeSchema = z.object({
	path: z.string().describe('Workspace path of the node to preview-run.'),
	args: z
		.record(z.string(), z.any())
		.nullable()
		.optional()
		.describe('Arguments to pass to the script. Omit or pass null when none are needed.')
})

const testPipelineNodeToolDef = createToolDef(
	testPipelineNodeSchema,
	'test_pipeline_node',
	'Preview-run one pipeline node (using its draft body when unsaved) and return the result/logs, without deploying. Requires user confirmation before it runs.',
	{ strict: false }
)

// ----------------------------------------------------------------------------
// Tool set
// ----------------------------------------------------------------------------

export const pipelineTools: Tool<PipelineToolHelpers>[] = [
	{
		def: getPipelineGraphToolDef,
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			toolCallbacks.setToolStatus(toolId, { content: 'Reading pipeline graph...' })
			const ctx = pipeline.getPipelineContext()
			toolCallbacks.setToolStatus(toolId, {
				content: `Read pipeline graph (${ctx.nodes.length} node${ctx.nodes.length === 1 ? '' : 's'})`,
				result: 'Success'
			})
			return JSON.stringify(ctx, null, 2)
		}
	},
	{
		def: readPipelineNodeToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			const { path } = readPipelineNodeSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Reading node '${path}'...` })
			const node = await pipeline.getNodeBody(path)
			if (!node) {
				return `No pipeline node found at '${path}'. Call get_pipeline_graph to list the available nodes.`
			}
			toolCallbacks.setToolStatus(toolId, { content: `Read node '${path}'`, result: 'Success' })
			return JSON.stringify({ path, language: node.language, content: node.content })
		}
	},
	{
		def: buildPipelineNodeToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			const { path, language, content, output_kind } = buildPipelineNodeSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Building node '${path}'...` })
			await pipeline.proposeNode({
				path,
				language: language as ScriptLang,
				content,
				outputKind: output_kind as PipelineOutputKind | undefined
			})
			toolCallbacks.setToolStatus(toolId, {
				content: `Staged node '${path}' for review`,
				result: 'Success'
			})
			return `Pipeline node '${path}' staged as an AI-pending draft. It is shown on the canvas for the user to Accept or Reject; it is not deployed.`
		}
	},
	{
		def: editPipelineNodeToolDef,
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			const { path, old_string, new_string, replace_all } = editPipelineNodeSchema.parse(args)
			const node = await pipeline.getNodeBody(path)
			if (!node) {
				return `No pipeline node found at '${path}'. Call get_pipeline_graph to list the available nodes.`
			}
			toolCallbacks.setToolStatus(toolId, { content: `Editing node '${path}'...` })
			const updated = findAndReplace(
				node.content,
				old_string,
				new_string,
				replace_all ?? false,
				'node source'
			)
			await pipeline.editNode(path, updated)
			toolCallbacks.setToolStatus(toolId, {
				content: `Staged edit to '${path}' for review`,
				result: 'Success'
			})
			return `Pipeline node '${path}' updated and staged as an AI-pending draft for the user to Accept or Reject.`
		}
	},
	{
		def: removePipelineNodeToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			const { path } = removePipelineNodeSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Removing staged node '${path}'...` })
			await pipeline.removeProposedNode(path)
			toolCallbacks.setToolStatus(toolId, {
				content: `Removed staged node '${path}'`,
				result: 'Success'
			})
			return `Removed AI-pending node '${path}' from the staged set.`
		}
	},
	{
		def: testPipelineNodeToolDef,
		requiresConfirmation: true,
		confirmationMessage: 'Run pipeline node',
		showDetails: true,
		autoCollapseDetails: false,
		fn: async ({ args, workspace, helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			const { path, args: runArgs } = testPipelineNodeSchema.parse(args)
			return executeTestRun({
				jobStarter: async () => {
					const jobId = await pipeline.testNode(path, runArgs ?? undefined)
					if (!jobId) {
						throw new Error(`Could not start a run for node '${path}'.`)
					}
					return jobId
				},
				workspace,
				toolCallbacks,
				toolId,
				startMessage: `Starting run of '${path}'...`,
				contextName: 'script'
			})
		}
	}
]

/**
 * Pipeline-specific guidance appended to the global system prompt when a
 * /pipeline editor is open. Describes the annotation model and the
 * build-with-diff/approval workflow so the model uses the staging tools rather
 * than the generic write_script draft tools.
 */
export function getPipelinePromptSection(ctx: PipelineContext): string {
	return `

Data Pipeline editor (ACTIVE):
- The user has the /pipeline/${ctx.folder} editor open. A pipeline is a DAG of scripts (nodes) connected by storage assets (DuckLake tables, data tables, S3 objects, volumes, resources) and execution triggers.
- A script becomes a pipeline node when its source starts with the \`// pipeline\` annotation. Declare execution-DAG inputs with \`// on <asset-uri | schedule | webhook | email | kafka | mqtt | nats | postgres | sqs | gcp | data_upload>\` (e.g. \`// on ducklake://main/orders\`). Outputs are inferred from what the body writes (wmill SDK calls / SQL CREATE TABLE / writeS3File); declare a managed output with \`// materialize <asset-uri>\`. Optional badges: \`// partitioned <daily|hourly|weekly|monthly|dynamic>\`, \`// freshness <duration>\`, \`// tag <name>\`, \`// retry <count> [delay]\`, \`// data_test <kind> ...\`.
- Use get_pipeline_graph to see the current nodes/assets/triggers, and read_pipeline_node before editing one.
- Build new nodes with build_pipeline_node and edit existing ones with edit_pipeline_node. These DO NOT deploy — they stage an AI-pending draft that appears on the canvas highlighted for the user to Accept (keep) or Reject (discard), like the flow editor's diff review. Prefer these over the generic write_script/edit_script draft tools while a pipeline is open.
- Reuse existing asset paths from the graph when wiring a downstream node to an upstream one (read the upstream's write asset, then \`// on\` that same URI).
- Only deploy when the user explicitly asks; the user deploys staged drafts from the canvas.`
}
