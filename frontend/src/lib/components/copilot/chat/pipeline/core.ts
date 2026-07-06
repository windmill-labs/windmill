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
// (scripts annotated with `// pipeline`). Mutations don't deploy: they apply
// directly as an unsaved DRAFT on the canvas — the same way the flow/script
// editor applies AI edits — which the user then deploys. There is no separate
// approve/reject step: the draft IS the change.
//
// The pipeline tools are added on top of the full global tool set, so docs
// search, datatable SQL, and workspace-item tools are already available
// alongside them — this file only carries the pipeline-graph-specific surface.
// ============================================================================

/** Compact, model-facing summary of one node in the pipeline graph. */
export type PipelineNodeSummary = {
	path: string
	language?: ScriptLang
	/** Has an unsaved local edit (draft) not yet deployed. */
	unsaved: boolean
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
}

/**
 * Bridge the pipeline page registers on the AIChatManager. Reads expose the live
 * graph; writes apply directly as unsaved drafts (never deploy). Kept intentionally
 * small — the page owns the draft Map and canvas rendering.
 */
export interface PipelineAIChatHelpers {
	getPipelineContext: () => PipelineContext
	/** Read a node's source (the in-flight draft body if one exists, else deployed). */
	getNodeBody: (path: string) => Promise<{ language: ScriptLang; content: string } | undefined>
	/** Create a brand-new pipeline node as an unsaved draft on the canvas. */
	proposeNode: (input: {
		path: string
		language: ScriptLang
		content: string
		outputKind?: PipelineOutputKind
	}) => Promise<{ path: string }>
	/** Replace an existing node's body, applied as an unsaved draft. */
	editNode: (path: string, content: string) => Promise<void>
	/** Discard the unsaved draft at a path (undo a build_pipeline_node). */
	removeProposedNode: (path: string) => Promise<void>
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
	"Read the live pipeline graph for the open /pipeline/<folder> editor: its nodes (scripts), each node's language, asset reads/writes, declared triggers, and whether it has an unsaved draft edit. Call this before building or editing nodes so you reuse existing assets/paths and understand the current DAG."
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
// Mutation tools (apply directly as unsaved drafts; never deploy)
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
			"Full script source. Start it with the `pipeline` annotation as a top-of-file comment in the LANGUAGE'S comment syntax — `-- pipeline` for SQL (duckdb/postgresql), `# pipeline` for python3/bash, `// pipeline` for bun/TS — to mark it a pipeline member; declare inputs the same way (e.g. `-- on <asset-uri|schedule|webhook|...>`), and write outputs via the wmill SDK / SQL so the lineage edges are inferred. A `// pipeline` line in a SQL node is a syntax error. Read existing node bodies first to match conventions."
		),
	output_kind: outputKindSchema.optional()
})

const buildPipelineNodeToolDef = createToolDef(
	buildPipelineNodeSchema,
	'build_pipeline_node',
	'Build a NEW pipeline node. It is applied directly as an unsaved draft on the canvas (a dashed node wired by its parsed asset reads/writes) — it does NOT deploy; the user deploys it. Prefer this over editing for new scripts.',
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
	'Edit an existing pipeline node by exact find/replace. The result is applied directly as an unsaved draft (does NOT deploy). Call read_pipeline_node first to get the exact current text.',
	{ strict: false }
)

const removePipelineNodeSchema = z.object({
	path: z.string().describe('Workspace path of the node whose unsaved draft should be discarded.')
})

const removePipelineNodeToolDef = createToolDef(
	removePipelineNodeSchema,
	'remove_pipeline_node',
	'Discard the unsaved draft at a path (undo a build_pipeline_node). Only affects the in-flight draft — to delete a deployed node, ask the user to do it on the canvas.'
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
				content: `Added draft node '${path}'`,
				result: 'Success'
			})
			return `Pipeline node '${path}' added as an unsaved draft on the canvas. It is not deployed — the user deploys it.`
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
				content: `Edited draft '${path}'`,
				result: 'Success'
			})
			return `Pipeline node '${path}' updated as an unsaved draft on the canvas (not deployed).`
		}
	},
	{
		def: removePipelineNodeToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const pipeline = requirePipeline(helpers)
			const { path } = removePipelineNodeSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Discarding draft '${path}'...` })
			await pipeline.removeProposedNode(path)
			toolCallbacks.setToolStatus(toolId, {
				content: `Discarded draft '${path}'`,
				result: 'Success'
			})
			return `Discarded the unsaved draft at '${path}'.`
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
 * /pipeline editor is open. Describes the annotation model and the direct-draft
 * workflow so the model uses the pipeline tools rather than the generic
 * write_script draft tools.
 */
export function getPipelinePromptSection(ctx: PipelineContext): string {
	return `

Data Pipeline editor (ACTIVE):
- The user has the /pipeline/${ctx.folder} editor open. A pipeline is a DAG of scripts (nodes) connected by storage assets (DuckLake tables, data tables, S3 objects, volumes, resources) and execution triggers.
- Annotations are top-of-file comments in the NODE'S OWN comment syntax: \`--\` for SQL (duckdb/postgresql), \`#\` for python3/bash, \`//\` for bun/TS. The \`//\` shown below is the TS form — translate it (a \`// pipeline\` line in a SQL node is a syntax error that won't deploy).
- A script becomes a pipeline node when its source starts with the \`// pipeline\` annotation. Declare execution-DAG inputs with \`// on <asset-uri | schedule | webhook | email | kafka | mqtt | nats | postgres | sqs | gcp | data_upload>\` (e.g. \`// on ducklake://main/orders\`). Outputs are inferred from what the body writes (wmill SDK calls / SQL CREATE TABLE / writeS3File); declare a managed output with \`// materialize <asset-uri>\`. Optional badges: \`// partitioned <daily|hourly|weekly|monthly|dynamic>\`, \`// freshness <duration>\`, \`// tag <name>\`, \`// retry <count> [delay]\`, \`// data_test <kind> ...\`.
- \`materialize\` (the managed output): \`// materialize <asset-uri>\` means the runtime writes the node's output table FOR you — write the body as a single SELECT and the runtime wraps it in the create/replace, so do NOT also write your own CREATE TABLE / INSERT. IMPORTANT: \`// materialize\` is **DuckDB-only** and its target MUST be a DuckLake table (\`ducklake://<name>/<table>\`) — deploy rejects it on any other language or target. For a \`python3\`/\`bun\`/\`postgresql\` node, do NOT use \`// materialize\`; write the output via the SDK instead (e.g. \`wmill.writeS3File(...)\`, a \`CREATE TABLE\` in postgresql, or \`wmill.databaseUrlFromResource\`/ducklake helpers) and let the output be inferred. Reach for \`duckdb\` when a node should materialize a DuckLake table. Write strategy: with no option it REPLACES the whole table each run (full refresh; the only mode whose output columns may change); \`// materialize <uri> append\` INSERT-appends rows (incremental); \`// materialize <uri> key=<col>\` merges/upserts on \`<col>\`. \`// materialize manual <uri>\` opts OUT of managed writes — the script writes its own DDL and the annotation only records the output asset for lineage. \`materialize\` is paired with partitioning for incremental pipelines: a \`// partitioned <daily|hourly|weekly|monthly|dynamic>\` node runs once per partition (append/merge into a fixed-schema table), and the \`{partition}\` token — usable in any asset URI AND in the body SQL — is substituted with the current partition's IDENTITY string at run time. To filter the source to the active slice on a time grain, compare with \`strftime\` in the format the runtime builds the identity from — \`WHERE strftime(<ts_col>, '<fmt>') = {partition}\` (fmt: daily \`%Y-%m-%d\`, hourly \`%Y-%m-%dT%H\`, weekly \`%G-W%V\`, monthly \`%Y-%m\`). Do NOT write \`= TIMESTAMP {partition}\`: the identity string is not a valid timestamp literal for hourly/weekly/monthly and errors at runtime (only daily happens to parse). For \`dynamic\` partitioning the identity is your caller-supplied key (not a timestamp), so filter on it directly: \`WHERE <your_key_col> = {partition}\`. \`materialize\` is an output DECLARATION on the node — it is not a command; there is no "materialize run".
- Use get_pipeline_graph to see the current nodes/assets/triggers, and read_pipeline_node before editing one.
- Build new nodes with build_pipeline_node and edit existing ones with edit_pipeline_node. These apply directly as unsaved drafts on the canvas (like the flow/script editor applies AI edits) — they DO NOT deploy. There is no separate Accept/Reject step. Prefer these over the generic write_script/edit_script draft tools while a pipeline is open.
- Reuse existing asset paths from the graph when wiring a downstream node to an upstream one (read the upstream's write asset, then \`// on\` that same URI).
- Only deploy when the user explicitly asks; the user deploys drafts from the canvas.`
}
