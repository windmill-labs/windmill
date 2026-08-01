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
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import { Button } from '../common'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import DbtProjectPanel from './DbtProjectPanel.svelte'
	import DbtModelGraph from './DbtModelGraph.svelte'
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
	import { CornerDownLeft, Github, Play, Plus } from 'lucide-svelte'
	import type { ScriptEditorWhitelabelCustomUi } from '../custom_ui'
	import { processSecretArgs } from '../secretArgUtils'

	let {
		schema = $bindable(),
		code = $bindable(),
		args = $bindable(),
		modules = $bindable(undefined),
		editor = $bindable(undefined),
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

	let jobLoader: JobLoader | undefined = $state(undefined)
	let testJob: any = $state(undefined)
	let testIsLoading = $state(false)
	let logPanel: LogPanel | undefined = $state(undefined)
	let rightTab = $state<'models' | 'run'>('models')

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
		// A build's output is the point of running it, and the log panel lives
		// under the other tab.
		rightTab = 'run'
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
		{#if customUi?.editorBar?.useVsCode != false}
			<Button
				target="_blank"
				href="https://www.windmill.dev/docs/cli_local_dev/vscode-extension"
				variant="subtle"
				unifiedSize="sm"
				title="Edit this project locally"
				startIcon={{ icon: Github }}
			>
				VScode
			</Button>
		{/if}
		{#if testIsLoading}
			<Button on:click={() => jobLoader?.cancelJob()} unifiedSize="sm">
				<WindmillIcon white={true} class="mr-2 text-white" height="16px" width="20px" spin="fast" />
				Cancel
			</Button>
		{:else}
			<Button
				on:click={() => runTest()}
				unifiedSize="sm"
				variant="accent-secondary"
				startIcon={{ icon: Play, classes: 'animate-none' }}
				shortCut={{ Icon: CornerDownLeft }}
			>
				<!-- Named, because a run that silently narrowed to whichever file happens
				     to be open is the kind of surprise a warehouse bill discovers. -->
				{selected ? `Build ${selected.name}` : 'Build project'}
			</Button>
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
			<div class="flex flex-col h-full">
				<Tabs bind:selected={rightTab}>
					<Tab value="models" label="Models" />
					<Tab value="run" label="Run" />
				</Tabs>
				<!-- Both panes stay MOUNTED and are hidden rather than swapped: each
				     holds state a tab switch has no business ending. The graph holds
				     the parse it is pinned to, and a refresh in flight — unmounting
				     it drops the pin back to the deployed graph and abandons a job
				     that costs a worker slot. The log panel holds a running build. -->
				<div class="flex-1 min-h-0" class:hidden={rightTab !== 'models'}>
					<DbtModelGraph
						workspace={opWs}
						scriptPath={path ?? ''}
						descriptor={code}
						modules={modules ?? undefined}
						{args}
						{tag}
						{timeout}
						{deployedHash}
						onOpenFile={open}
					/>
				</div>
				<div class="flex-1 min-h-0" class:hidden={rightTab !== 'run'}>
					<Splitpanes horizontal class="h-full">
						<Pane size={40} minSize={15}>
							<div class="p-2 overflow-auto h-full">
								{#if schema}
									<SchemaForm {schema} bind:args noVariablePicker={false} showSchemaExplorer />
								{/if}
							</div>
						</Pane>
						<Pane size={60} minSize={20} class="relative">
							<LogPanel
								bind:this={logPanel}
								workspace={opWs}
								lang={'dbt' as Preview['language']}
								previewJob={testJob}
								previewIsLoading={testIsLoading}
								{editor}
								{args}
								showCaptures={false}
								customUi={customUi?.previewPanel}
							/>
						</Pane>
					</Splitpanes>
				</div>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
