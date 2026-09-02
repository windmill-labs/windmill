<script lang="ts">
	// The editor for a dbt script, which is a PROJECT rather than a body of code:
	// a file tree, a descriptor, run arguments and a model graph.
	//
	// The generic script editor accommodated all four by bolting a tree onto its
	// module tab strip, but its premise — one file, one signature, one body — is
	// the wrong one here: dbt resolves `ref()` project-wide and cannot run a
	// single file, so the arguments, the run and the graph are all the project's
	// whichever file happens to be open.
	import { untrack } from 'svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import type { Schema, SupportedLanguage } from '$lib/common'
	import type { Preview, ScriptModule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema } from '$lib/utils'
	import { inferArgs } from '$lib/infer'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Editor from '../Editor.svelte'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import SchemaForm from '../SchemaForm.svelte'
	import LogPanel from '../scriptEditor/LogPanel.svelte'
	import JobLoader from '../JobLoader.svelte'
	import { Button } from '../common'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import DbtProjectPanel from './DbtProjectPanel.svelte'
	import DbtModelGraph from './DbtModelGraph.svelte'
	import DbtModelDetails from './DbtModelDetails.svelte'
	import type { DbtPreviewBuffer } from './previewRows'
	import type {
		AssetGraphNodeData,
		DbtAssetProvenance
	} from '$lib/components/assets/AssetGraph/types'
	import {
		DBT_DESCRIPTOR,
		DBT_MODULE_EXTENSIONS,
		dbtDefaultContent,
		dbtDescriptorSummary,
		dbtFileLang,
		dbtModelSelector,
		dbtModuleLang,
		dbtModulePath,
		dbtPathError
	} from './projectFiles'
	import { ChevronDown, CornerDownLeft, Play, Plus } from 'lucide-svelte'
	import type { ScriptEditorWhitelabelCustomUi } from '../custom_ui'
	import { processSecretArgs } from '../secretArgUtils'

	let {
		schema = $bindable(),
		code = $bindable(),
		args = $bindable(),
		modules = $bindable(),
		editor = $bindable(),
		path,
		tag,
		timeout = undefined,
		customUi = undefined,
		/** The deployed version, when the script has one. The Models panel draws it
		 *  until a refresh replaces it with the buffer's own. */
		deployedHash = undefined,
		onTestJob,
		workspaceOverride = undefined
	}: {
		schema?: Schema | any
		code: string
		args: Record<string, any>
		modules?: { [key: string]: ScriptModule } | null
		editor?: Editor | undefined
		path: string | undefined
		tag: string | undefined
		timeout?: number
		customUi?: ScriptEditorWhitelabelCustomUi | undefined
		deployedHash?: string | number
		onTestJob?: (e: { jobId: string }) => void
		workspaceOverride?: string
	} = $props()

	const dispatch = createEventDispatcher()
	let opWs = $derived(workspaceOverride ?? $workspaceStore)

	/** The open file, or `null` for the descriptor at the project root. */
	let openFile = $state<string | null>(null)
	/** What the editor shows: the descriptor, or one project file. */
	let editorCode = $state(code)
	let lastSyncedCode = code

	// The content prop moves under the editor on a template seed, a draft restore
	// or an AI edit. Only meaningful while the descriptor is what is open — a
	// project file's content lives in `modules`.
	$effect.pre(() => {
		if (openFile === null && code !== lastSyncedCode) {
			editorCode = code
			lastSyncedCode = code
			editor?.setCode(editorCode)
			untrack(() => inferSchema(code))
		}
	})

	function flushOpenFile() {
		if (openFile !== null && modules?.[openFile]) {
			modules[openFile] = { ...modules[openFile], content: editorCode }
		}
	}

	function open(file: string | null) {
		flushOpenFile()
		openFile = file
		editorCode = file === null ? code : (modules?.[file]?.content ?? '')
		if (file === null) lastSyncedCode = code
		editor?.setCode(editorCode)
	}

	// The whole surface ScriptBuilder drives an editor through. `flushModuleState`
	// and `disableCollaboration` are the two it calls before saving and on
	// language change.
	export function setArgs(nargs: Record<string, any>) {
		args = nargs
	}
	export function updateArgs(nargs: Record<string, any>) {
		args = { ...args, ...nargs }
	}
	export function flushModuleState() {
		flushOpenFile()
	}
	/** A dbt project is a bundle of files, and the collaborative session only ever
	 *  covered a single body, so this editor opens none to disable. Kept because
	 *  the builder calls it before switching language. */
	export function disableCollaboration() {}

	/** The run form's arguments, which are the DESCRIPTOR's: the command block and
	 *  one entry per `{{ placeholder }}` it interpolates. Derived from the
	 *  descriptor whatever file is open, because that is what a run takes.
	 *
	 *  `nlang` is honored rather than assumed to be dbt: the builder calls this on
	 *  the editor that is still mounted when the language CHANGES, so the content
	 *  here can already be the next language's template. */
	export async function inferSchema(
		content: string,
		{ nlang, resetArgs = false }: { nlang?: SupportedLanguage; resetArgs?: boolean } = {}
	) {
		// Before the parse, not after it: a reset means the arguments describe a
		// script that no longer exists, and leaving them for a parse to clear
		// hands dbt's command block to whatever editor mounts next when it fails.
		if (resetArgs) args = {}
		let nschema = schema ?? emptySchema()
		try {
			await inferArgs(nlang ?? 'dbt', content, nschema)
			validDescriptor = true
			descriptorError = undefined
			schema = nschema
		} catch (e) {
			validDescriptor = false
			// `inferArgs` reports a dbt parse failure with an empty message, so what
			// it says is kept only when there is something to say and the deploy
			// remains where the reason comes from.
			const said = e instanceof Error ? e.message : String(e)
			descriptorError = said.trim() || undefined
		}
	}

	let validDescriptor = $state(true)
	let descriptorError = $state<string | undefined>(undefined)
	let summary = $derived(dbtDescriptorSummary(code))

	// The selector a run would narrow to, when the open file is a model. Macros,
	// analyses and singular tests are `.sql` too and none is selectable by name,
	// so those fall back to building the project.
	let selected = $derived.by(() => {
		const file = openFile
		if (!file) return undefined
		const selector = dbtModelSelector(modules ?? {}, file)
		if (!selector) return undefined
		return {
			selector,
			name: file
				.split('/')
				.pop()!
				.replace(/\.(sql|py)$/, '')
		}
	})

	// Whether the run arguments still say what the descriptor does. Only the
	// command block counts: a `{{ placeholder }}` is a value the project asks for
	// on every run, not an override of anything.
	let argsOverridden = $derived.by(() => {
		const cmd = args?.command as Record<string, unknown> | undefined
		if (!cmd) return false
		const dflt = (schema?.properties?.command?.default ?? {}) as Record<string, unknown>
		return Object.keys(cmd).some((k) => JSON.stringify(cmd[k]) !== JSON.stringify(dflt[k]))
	})

	// Neither history nor tracing belongs beside a dbt build: the run form's own
	// history is the deployed script's, and a build is one invocation rather than
	// a flow with steps to trace. With those gone `LogPanel` is a single tab, and
	// it drops its own tab strip rather than showing a bar of one.
	let logPanelUi = $derived({
		...(customUi?.previewPanel ?? {}),
		disableHistory: true,
		disableTracing: true
	})

	// The node picked on the graph, if any. It decides what the bottom section is:
	// a model's details while one is selected, the run's logs otherwise. Held here
	// rather than in the graph because closing the details has to deselect, and
	// both halves are this component's to place.
	let graphSelection = $state<AssetGraphNodeData | undefined>(undefined)
	let selectedAsset = $derived(graphSelection?.kind === 'asset' ? graphSelection : undefined)
	let selectedDbt = $state<DbtAssetProvenance | undefined>(undefined)
	// Set when the selected node came from a buffer parse: the project that parse
	// ran on, which is the one its rows must come from. Undefined for a node off
	// the deployed graph, which previews by version instead. Either way the rows
	// come from the project whose SQL is displayed above them.
	let selectedBuffer = $state<DbtPreviewBuffer | undefined>(undefined)

	let jobLoader: JobLoader | undefined = $state(undefined)
	let testJob: any = $state(undefined)
	let testIsLoading = $state(false)
	let logPanel: LogPanel | undefined = $state(undefined)

	// The options are the generic editor's, so the two stay interchangeable to the
	// builder. Neither applies here: a dbt run dispatches nothing to cascade, and
	// a descriptor holds no DDL to guard.
	export async function runTest(_opts?: { cascade?: boolean; skipDdlGuard?: boolean }) {
		flushOpenFile()
		const testArgs = await processSecretArgs(args ?? {}, schema, opWs)
		// Building with a model open builds THAT model: `dbt build --select
		// <model>` is dbt's own inner loop, and running the whole project to check
		// one file is the thing a dbt developer never does. Its tests come along,
		// because `build` interleaves them.
		if (selected) {
			testArgs.command = {
				...((testArgs.command as object) ?? {}),
				label: 'build',
				select: [selected.selector]
			}
		}
		const job = await jobLoader?.runPreview(
			path,
			code,
			'dbt' as Preview['language'],
			testArgs,
			tag,
			undefined,
			undefined,
			undefined,
			undefined,
			// The whole bundle, always: it IS the project, and without it the run
			// finds no `dbt_project.yml` whichever file happens to be open.
			modules,
			undefined,
			timeout
		)
		if (job) onTestJob?.({ jobId: job })
		logPanel?.setFocusToLogs()
		return job
	}

	let newFile = $state('')
	let newFileError = $state<string | undefined>(undefined)
	let newFileInput: HTMLInputElement | undefined = $state(undefined)
	let addOpen = $state(false)

	function createFile() {
		// Keyed by the CANONICAL path, so `./models/x.sql` and `models/x.sql`
		// cannot become two keys for one file in the bundle the worker writes.
		const resolved = dbtModulePath(newFile, modules ?? undefined)
		if ('error' in resolved) {
			newFileError = resolved.error
			return
		}
		const p = resolved.path
		modules = {
			...(modules ?? {}),
			[p]: { content: dbtDefaultContent(p), language: dbtModuleLang(p)! }
		}
		newFile = ''
		newFileError = undefined
		addOpen = false
		open(p)
	}

	function removeFile(p: string) {
		if (!modules?.[p]) return
		const { [p]: _dropped, ...rest } = modules
		modules = rest
		if (openFile === p) open(null)
	}

	function onKeyDown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
			e.preventDefault()
			void runTest()
		}
	}

	onDestroy(() => {
		flushOpenFile()
	})

	let fileLang = $derived<Preview['language']>(openFile ? dbtFileLang(openFile) : 'ansible')
</script>

<JobLoader
	noCode={true}
	workspaceOverride={opWs}
	bind:this={jobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window onkeydown={onKeyDown} />

<div class="border-b shadow-sm px-2 py-1 flex items-center gap-3 text-2xs">
	<span class="font-mono text-secondary truncate">{path ?? ''}__dbt/</span>
	<span class="text-tertiary shrink-0">{summary.engine}</span>
	<span
		class="text-tertiary shrink-0"
		title="The workspace warehouse this project's assets are keyed on"
		>warehouse: {summary.warehouse}</span
	>
	{#if !validDescriptor}
		<!-- The descriptor drives warehouse writes, so an unknown field is refused
		     rather than defaulted — and a deploy or a refresh would fail on it. -->
		<span class="text-red-500 truncate" title={descriptorError}>
			{descriptorError ? `descriptor: ${descriptorError}` : 'descriptor is not valid'}
		</span>
	{/if}
	<div class="ml-auto shrink-0 flex items-center gap-2">
		{#if testIsLoading}
			<Button on:click={() => jobLoader?.cancelJob()} unifiedSize="sm">
				<WindmillIcon white={true} class="mr-2 text-white" height="16px" width="20px" spin="fast" />
				Cancel
			</Button>
		{:else}
			<div class="flex items-center">
				<Button
					on:click={() => runTest()}
					unifiedSize="sm"
					variant="accent-secondary"
					startIcon={{ icon: Play, classes: 'animate-none' }}
					shortCut={{ Icon: CornerDownLeft }}
					btnClasses="!rounded-r-none"
				>
					<!-- Named, because a run that silently narrowed to whichever file happens
					     to be open is the kind of surprise a warehouse bill discovers. -->
					{selected ? `Build ${selected.name}` : 'Build project'}
				</Button>
				<!-- The command block lives here rather than in the panel: it is a run
				     CONFIGURATION, set once and then built from repeatedly, and it was
				     taking permanent vertical space from the two things you actually
				     watch. The dot says it is no longer the descriptor's own. -->
				<!-- The Popover renders its OWN `<button>`, so the trigger is styled
				     rather than filled with another one: a nested button is invalid and
				     swallows the click. `h-7` is the accent-secondary `sm` Button's
				     height, which is what makes the pair read as one control. -->
				<Popover
					placement="bottom-end"
					contentClasses="p-3 w-[30rem] max-h-[70vh] overflow-auto"
					class="relative h-7 px-1.5 flex items-center rounded-r-md border border-l-0 border-blue-500/60 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-950/40"
					triggerAttrs={{ title: 'Build arguments' }}
				>
					{#snippet trigger()}
						<ChevronDown size={14} />
						{#if argsOverridden}
							<span
								class="pointer-events-none absolute top-0.5 right-0.5 w-1.5 h-1.5 rounded-full bg-blue-500"
							></span>
						{/if}
					{/snippet}
					{#snippet content()}
						{#if schema}
							<SchemaForm
								{schema}
								bind:args
								workspace={opWs}
								noVariablePicker={false}
								showSchemaExplorer
							/>
						{:else}
							<p class="text-2xs text-tertiary">This descriptor takes no arguments.</p>
						{/if}
					{/snippet}
				</Popover>
			</div>
		{/if}
	</div>
</div>

<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane size={62} minSize={20} class="!overflow-visible">
			<div class="h-full flex flex-row bg-surface dark:bg-surface-secondary !overflow-visible">
				<DbtProjectPanel
					modules={modules ?? {}}
					scriptPath={path ?? ''}
					descriptorName={DBT_DESCRIPTOR}
					selected={openFile}
					onSelect={open}
					onDelete={removeFile}
				>
					{#snippet addFile()}
						<Popover
							bind:isOpen={addOpen}
							placement="bottom-end"
							openFocus={newFileInput}
							contentClasses="p-3 w-72"
						>
							{#snippet trigger()}
								<div class="p-0.5 rounded hover:bg-surface-hover" title="New file">
									<Plus size={12} />
								</div>
							{/snippet}
							{#snippet content({ close })}
								<div class="flex flex-col gap-2">
									<label for="dbt-new-file" class="text-xs font-semibold text-emphasis">
										File name
									</label>
									<input
										id="dbt-new-file"
										type="text"
										class="border rounded px-2 py-1.5 text-sm bg-surface"
										bind:this={newFileInput}
										bind:value={newFile}
										placeholder="models/staging/stg_orders.sql"
										oninput={() => (newFileError = dbtPathError(newFile, modules ?? undefined))}
										onkeydown={(e) => {
											if (e.key === 'Enter') createFile()
											if (e.key === 'Escape') close()
										}}
									/>
									{#if newFileError}
										<p class="text-red-500 text-2xs">{newFileError}</p>
									{/if}
									<p class="text-tertiary text-2xs">
										Anywhere in the project, e.g. <code class="text-2xs">macros/cents.sql</code>.
										{DBT_MODULE_EXTENSIONS.join(', ')}
									</p>
									<div class="flex justify-end gap-2">
										<Button
											variant="default"
											size="xs"
											onclick={() => {
												newFile = ''
												newFileError = undefined
												close()
											}}>Cancel</Button
										>
										<Button
											variant="accent"
											size="xs"
											onclick={createFile}
											disabled={!newFile.trim() || !!newFileError}>Add</Button
										>
									</div>
								</div>
							{/snippet}
						</Popover>
					{/snippet}
				</DbtProjectPanel>
				<div class="relative flex-1 min-h-0 min-w-0 !overflow-visible">
					{#key fileLang}
						<Editor
							lineNumbersMinChars={4}
							folding
							{path}
							bind:code={editorCode}
							bind:this={editor}
							on:change={() => {
								if (openFile === null) {
									code = editorCode
									lastSyncedCode = code
									inferSchema(editorCode)
								} else {
									flushOpenFile()
								}
							}}
							on:saveDraft
							cmdEnterAction={async () => {
								if (openFile === null) await inferSchema(editorCode)
								runTest()
							}}
							formatAction={async () => {
								if (openFile === null) await inferSchema(editorCode)
								dispatch('format')
							}}
							class="flex flex-1 h-full !overflow-visible"
							scriptLang={fileLang}
							automaticLayout={true}
							fixedOverflowWidgets={true}
							{args}
							customTag={tag}
						/>
					{/key}
				</div>
			</div>
		</Pane>
		<Pane size={38} minSize={20}>
			<!-- One pane, not two tabs. A build's models are what you want to watch
			     while it runs, and a tab makes seeing the graph and the run a choice
			     between them; the graph colours from the build instead. -->
			<Splitpanes horizontal class="h-full">
				<Pane size={50} minSize={20}>
					<DbtModelGraph
						workspace={opWs}
						scriptPath={path ?? ''}
						descriptor={code}
						modules={modules ?? undefined}
						{args}
						{tag}
						{timeout}
						{deployedHash}
						testJobId={testJob?.id}
						testRunning={testIsLoading}
						testResult={testJob?.result}
						selection={graphSelection}
						onSelect={(sel, dbt, buffer) => {
							graphSelection = sel
							selectedDbt = dbt
							selectedBuffer = buffer
						}}
					/>
				</Pane>
				<Pane size={50} minSize={15} class="relative">
					<!-- Selecting a node takes this whole section: a model's relation,
					     its declared columns and tests, its SQL and its actual rows do
					     not fit under the canvas, and the close button is what puts the
					     logs back — deselecting the node with it. The log panel is kept
					     MOUNTED underneath so a build running while a node is selected
					     does not lose its stream. -->
					{#if selectedAsset && selectedDbt}
						<DbtModelDetails
							workspace={opWs}
							scriptPath={path ?? ''}
							assetPath={selectedAsset.path}
							dbt={selectedDbt}
							scriptHash={deployedHash}
							buffer={selectedBuffer}
							{args}
							fileInBundle={!!selectedDbt.original_file_path &&
								!!modules?.[selectedDbt.original_file_path]}
							onOpenFile={open}
							onClose={() => (graphSelection = undefined)}
						/>
					{/if}
					<div class="h-full" class:hidden={selectedAsset && selectedDbt}>
						<LogPanel
							bind:this={logPanel}
							workspace={opWs}
							lang={'dbt' as Preview['language']}
							previewJob={testJob}
							previewIsLoading={testIsLoading}
							{editor}
							{args}
							showCaptures={false}
							customUi={logPanelUi}
						/>
					</div>
				</Pane>
			</Splitpanes>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
