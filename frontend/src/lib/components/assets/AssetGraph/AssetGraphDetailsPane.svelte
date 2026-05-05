<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'
	import { resource } from 'runed'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatAssetKind } from '$lib/components/assets/lib'
	import {
		AlertTriangle,
		Code2,
		ExternalLink,
		GitBranch,
		Loader2,
		Save,
		Trash2,
		X,
		Pencil
	} from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { tick } from 'svelte'
	import { inferArgs } from '$lib/infer'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import type { AssetGraphSelection } from './types'
	import { parsePipelineAnnotations, type PipelineAnnotations } from './parsePipelineAnnotations'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import S3FilePreview from '$lib/components/S3FilePreview.svelte'
	import DataTablePreview from './DataTablePreview.svelte'
	import AssetRunsPanel from './AssetRunsPanel.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { fade } from 'svelte/transition'
	import { userStore } from '$lib/stores'

	interface Props {
		// Regular selection — loads the script by path for inline editing.
		selection?: AssetGraphSelection | undefined
		// Pre-built draft script not yet persisted. Takes precedence over
		// `selection` when present. Used by the pipeline + menu so a new
		// pipeline script opens inline instead of navigating to /scripts/add.
		draftScript?: Script | undefined
		workspace: string
		onclose: () => void
		// Called after a draft is saved for the first time so the page can
		// refetch the graph and clear its local draft.
		onDraftSaved?: (savedPath: string) => void
		// Called when the user hits "Discard" on the draft header. Separate
		// from `onclose` so the page can drop the draft from its map vs
		// just dismissing the pane.
		onDiscard?: () => void
		// Emits live-parsed pipeline annotations from the open script so the
		// canvas can overlay unsaved schedule / trigger nodes in real time.
		// Fires whenever the script content changes.
		onAnnotationsChange?: (scriptPath: string | undefined, annotations: PipelineAnnotations) => void
		// Called after the user moves/renames a persisted script via the
		// summary/path popover. The page repoints `selection` at `newPath`
		// and refetches the graph so the runnable node label updates.
		onScriptRenamed?: (oldPath: string, newPath: string) => void
		// Called after the user archives or deletes a persisted script. The
		// page should drop the selection (the script no longer exists in
		// the active graph) and refetch.
		onScriptRemoved?: (path: string) => void
		// Producers of the currently selected asset (script/flow paths that
		// write to it). Used by the runs panel to list job history. Pulled
		// from the graph by the parent page rather than re-derived here so
		// drafts stay consistent with what the canvas already shows.
		selectionProducers?: Array<{
			kind: 'script' | 'flow'
			path: string
			unsaved?: boolean
		}>
		// Bumped by the parent after dispatching a run so the runs panel
		// re-fetches the listing immediately (rather than waiting on its
		// background poll tick).
		runsRefreshKey?: any
		// Job id of the most recently dispatched run. Forwarded to the
		// runs panel so that clicking play auto-selects the new run.
		runsPendingJobId?: string | undefined
		// Folder-scoped non-editable prefix shown next to the suffix
		// editor when the user renames a draft (e.g. `f/<folder>/`). The
		// new path = pathPrefix + suffix.
		pathPrefix?: string
		// Called when the user renames a draft — the parent reseats the
		// path key in its drafts map and updates activeDraftPath. Returns
		// true on success; false on collision/validation failure so the
		// popover can keep itself open and surface the error inline.
		onDraftPathChange?: (oldPath: string, newPath: string) => boolean | string
	}
	let {
		selection,
		draftScript,
		workspace,
		onclose,
		onDraftSaved,
		onDiscard,
		onAnnotationsChange,
		onScriptRenamed,
		onScriptRemoved,
		selectionProducers = [],
		runsRefreshKey,
		runsPendingJobId,
		pathPrefix = '',
		onDraftPathChange
	}: Props = $props()

	// Bumped when the runs panel reports a watched job has reached a
	// terminal state. Drives S3FilePreview's refreshKey so the preview
	// re-checks existence after a producer run finishes — moves the
	// "not yet materialized" empty state to the actual preview without
	// requiring the user to re-click the asset.
	let previewRefreshKey = $state(0)

	// When `draftScript` is provided we bypass the fetch entirely and edit
	// it locally; saving calls ScriptService.createScript to deploy it.
	let scriptRes = resource(
		[() => workspace, () => selection, () => draftScript],
		async ([ws, sel, draft], _prev, { signal }) => {
			if (draft) return undefined
			if (!sel || sel.kind !== 'runnable' || sel.runnable_kind !== 'script') return undefined
			return await ScriptService.getScriptByPath({ workspace: ws, path: sel.path }, signal as any)
		}
	)

	// Local mutable copy. For drafts: seeded once from the incoming prop
	// (subsequent typing stays in `script`, not `draftScript`). For fetched
	// scripts: reset whenever the resource yields new data.
	let script = $state<Script | undefined>(undefined)
	$effect.pre(() => {
		if (draftScript) {
			script = structuredClone($state.snapshot(draftScript) as Script)
			return
		}
		const fresh = scriptRes.current
		script = fresh ? structuredClone($state.snapshot(fresh) as Script) : undefined
	})

	let args = $state<Record<string, any>>({})
	let saving = $state(false)
	let isDraft = $derived(draftScript != undefined)

	// Single trash-bin button opens one modal that exposes both Archive
	// (always available) and Delete permanently (admin-only). Archive is
	// the default destructive action; delete is reserved for irrecoverable
	// cleanup. Modal is lifted from ConfirmationModal's structure but
	// hand-rolled here because we need *two* confirm buttons, not one.
	let removeOpen = $state(false)
	let removing = $state(false)
	let canHardDelete = $derived(!!($userStore?.is_admin || $userStore?.is_super_admin))

	async function archive() {
		if (!script || !script.hash) return
		removing = true
		try {
			await ScriptService.archiveScriptByHash({ workspace, hash: script.hash })
			sendUserToast('Script archived')
			const removedPath = script.path
			removeOpen = false
			onScriptRemoved?.(removedPath)
		} catch (e: any) {
			sendUserToast(`Could not archive: ${e.body ?? e.message}`, true)
		} finally {
			removing = false
		}
	}

	async function deleteScript() {
		if (!script || !script.path) return
		removing = true
		try {
			// Path-based delete drops every version at the path. The hash-based
			// equivalent only soft-deletes the active row; for a "delete" the
			// user is asking for, the path-based call is the right semantic.
			await ScriptService.deleteScriptByPath({ workspace, path: script.path })
			sendUserToast('Script deleted')
			const removedPath = script.path
			removeOpen = false
			onScriptRemoved?.(removedPath)
		} catch (e: any) {
			sendUserToast(`Could not delete: ${e.body ?? e.message}`, true)
		} finally {
			removing = false
		}
	}

	// Live-parse the editor buffer so the page can render unsaved schedule /
	// trigger nodes on the canvas. Parsed TS output mirrors the Rust
	// `parse_pipeline_annotations` used at deploy time.
	let liveAnnotations = $derived<PipelineAnnotations>(
		script
			? parsePipelineAnnotations(script.content ?? '')
			: {
					inPipeline: false,
					triggerAssets: [],
					schedules: [],
					nativeTriggers: []
				}
	)
	$effect(() => {
		onAnnotationsChange?.(script?.path, liveAnnotations)
	})

	async function save() {
		if (!script) return
		saving = true
		try {
			script.schema = script.schema ?? emptySchema()
			try {
				const result = await inferArgs(script.language, script.content, script.schema)
				;(script as any).auto_kind = result?.auto_kind || undefined
				script.has_preprocessor = result?.has_preprocessor || undefined
			} catch {
				sendUserToast(`Could not parse code, are you sure it is valid?`, true)
			}
			await ScriptService.createScript({
				workspace,
				requestBody: {
					...script,
					language: script.language,
					description: script.description ?? '',
					// Drafts have no prior hash; workspace scripts chain off their last hash.
					parent_hash: isDraft || script.hash == undefined ? undefined : String(script.hash),
					is_template: false,
					tag: script.tag,
					kind: script.kind as Script['kind'] | undefined,
					lock: undefined
				}
			})
			sendUserToast(`Saved ${script.path}`)
			if (isDraft) {
				onDraftSaved?.(script.path)
			} else {
				await scriptRes.refetch()
			}
		} catch (e: any) {
			sendUserToast(`Save failed: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			saving = false
		}
	}

	let isScriptView = $derived(
		isDraft || (selection?.kind === 'runnable' && selection.runnable_kind === 'script')
	)

	// Suffix editor for the draft-path popover. Seeded from the current
	// path each time the popover opens so the user starts with what they
	// see, not stale state from an earlier rename.
	let draftPathSuffix = $state('')
	let draftPathError = $state<string | undefined>(undefined)
	let draftPathInput: HTMLInputElement | undefined = $state(undefined)

	function suffixOf(fullPath: string): string {
		return fullPath.startsWith(pathPrefix) ? fullPath.slice(pathPrefix.length) : fullPath
	}

	async function openDraftPathEditor() {
		draftPathSuffix = script ? suffixOf(script.path) : ''
		draftPathError = undefined
		await tick()
		draftPathInput?.focus()
		draftPathInput?.select()
	}

	function confirmDraftPath(close: () => void) {
		if (!script) return
		const suffix = draftPathSuffix.trim()
		if (!suffix) {
			draftPathError = 'Path cannot be empty'
			return
		}
		const newPath = pathPrefix + suffix
		if (newPath === script.path) {
			close()
			return
		}
		const result = onDraftPathChange?.(script.path, newPath)
		if (result === true || result === undefined) {
			script.path = newPath
			draftPathError = undefined
			close()
		} else if (typeof result === 'string') {
			draftPathError = result
		} else {
			draftPathError = 'Path already in use'
		}
	}
</script>

<div class="flex flex-col h-full bg-surface">
	<div
		class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0 min-h-10 whitespace-nowrap"
	>
		<div class="flex items-center gap-2 min-w-0">
			{#if isDraft && script}
				{@const draftScriptPath = script.path}
				<Code2 size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				{#if onDraftPathChange}
					<!-- Inline rename popover for drafts. The persisted-script
					     branch uses SummaryPathDisplay which round-trips through
					     updateItemPathAndSummary; drafts have no server row yet,
					     so we just rekey the parent's drafts map locally. -->
					<Popover
						placement="bottom-start"
						contentClasses="p-3"
						usePointerDownOutside
						on:openChange={(e) => {
							if (e.detail) openDraftPathEditor()
						}}
					>
						{#snippet trigger()}
							<button
								type="button"
								class="flex flex-col min-w-0 text-left px-2 py-1 rounded-md hover:bg-surface-hover transition-colors group"
								title="Edit draft path"
							>
								<span
									class="text-3xs uppercase tracking-wide text-emerald-600 dark:text-emerald-400 flex items-center gap-1"
								>
									Draft pipeline script
									<Pencil size={9} class="opacity-0 group-hover:opacity-60 transition-opacity" />
								</span>
								<span class="text-xs font-mono truncate" title={draftScriptPath}
									>{draftScriptPath}</span
								>
							</button>
						{/snippet}
						{#snippet content({ close })}
							<div class="flex flex-col gap-2 w-[420px]">
								<span class="text-2xs font-normal text-secondary">Path</span>
								<div
									class="flex items-stretch border rounded-md bg-surface overflow-hidden focus-within:ring-2 focus-within:ring-emerald-400"
								>
									{#if pathPrefix}
										<span
											class="flex items-center px-2 bg-surface-secondary text-tertiary text-sm font-mono border-r select-none"
										>
											{pathPrefix}
										</span>
									{/if}
									<input
										bind:this={draftPathInput}
										bind:value={draftPathSuffix}
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.preventDefault()
												confirmDraftPath(close)
											} else if (e.key === 'Escape') {
												e.preventDefault()
												close()
											}
										}}
										class="flex-1 min-w-0 px-2 py-1.5 text-sm font-mono bg-transparent focus:outline-none"
										placeholder="my_script"
									/>
								</div>
								{#if draftPathError}
									<span class="text-2xs text-red-500">{draftPathError}</span>
								{/if}
								<div class="flex justify-end">
									<Button
										variant="accent"
										unifiedSize="sm"
										disabled={!draftPathSuffix.trim()}
										onClick={() => confirmDraftPath(close)}
									>
										Rename
									</Button>
								</div>
							</div>
						{/snippet}
					</Popover>
				{:else}
					<div class="flex flex-col min-w-0">
						<span class="text-3xs uppercase tracking-wide text-emerald-600 dark:text-emerald-400">
							Draft pipeline script
						</span>
						<span class="text-xs font-mono truncate" title={script.path}>{script.path}</span>
					</div>
				{/if}
			{:else if selection?.kind === 'asset'}
				<AssetGenericIcon
					assetKind={selection.asset_kind}
					size="16px"
					class="shrink-0 text-blue-600 dark:text-blue-400"
				/>
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">
						{formatAssetKind({ kind: selection.asset_kind })}
					</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'script'}
				<Code2 size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<!-- Mirrors the rename UX from /scripts/get/[hash]: clicking the
				     path/summary opens an inline popover that calls
				     updateItemPathAndSummary. We forward `onSaved` upward so
				     the parent page can repoint its selection and refetch the
				     graph. Bound to the local `script` state so summary edits
				     reflect immediately without waiting on the resource. -->
				<SummaryPathDisplay
					summary={script?.summary ?? ''}
					path={selection.path}
					labels={script?.labels ?? []}
					kind="script"
					onSaved={(newPath) => {
						const oldPath = selection?.kind === 'runnable' ? selection.path : ''
						if (script) {
							script.path = newPath
						}
						onScriptRenamed?.(oldPath, newPath)
					}}
				/>
			{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'flow'}
				<GitBranch size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">Flow</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{/if}
		</div>
		<div class="flex items-center gap-1 shrink-0">
			{#if isDraft && onDiscard}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: Trash2 }}
					onclick={onDiscard}
					iconOnly
					title="Discard draft"
				/>
			{/if}
			<!-- Action order, left → right: trash, external link, save, close.
			     Save sits closest to Close so the primary commit action is
			     anchored at the right edge of the bar; trash lives at the
			     far left so destructive ops are visually separated from
			     navigation/commit. Mirrors the draft Discard placement. -->
			{#if !isDraft && isScriptView && script?.hash}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: Trash2 }}
					onclick={() => (removeOpen = true)}
					iconOnly
					title="Archive or delete"
				/>
			{/if}
			{#if !isDraft && selection?.kind === 'runnable'}
				<Button
					variant="subtle"
					unifiedSize="sm"
					href={selection.runnable_kind === 'flow'
						? `${base}/flows/edit/${selection.path}`
						: `${base}/scripts/edit/${selection.path}`}
					target="_blank"
					startIcon={{ icon: ExternalLink }}
					iconOnly
					title="Open in full editor"
				/>
			{/if}
			{#if isScriptView && script}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Save }}
					onclick={save}
					disabled={saving}
				>
					{saving ? 'Saving…' : isDraft ? 'Create' : 'Save'}
				</Button>
			{/if}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: X }}
				onclick={onclose}
				iconOnly
				title="Close"
			/>
		</div>
	</div>

	<div class="flex-1 min-h-0 relative">
		{#if selection?.kind === 'asset' && !isDraft}
			<!-- Vertical split: top pane is kind-specific (S3 has a content
			     preview; other kinds fall back to a navigational hint until
			     they grow their own previews); bottom pane is the runs panel,
			     which is generic — runs are keyed by producer script path,
			     not by asset kind, so it's useful for every asset. -->
			<Splitpanes horizontal class="!h-full">
				<Pane size={55} minSize={20}>
					{#if selection.asset_kind === 's3object'}
						<S3FilePreview
							fileKey={selection.path}
							showMetadata
							class="h-full"
							refreshKey={previewRefreshKey}
						/>
					{:else if selection.asset_kind === 'datatable'}
						<DataTablePreview path={selection.path} class="h-full" refreshKey={previewRefreshKey} />
					{:else}
						<div class="p-3 text-xs text-secondary">
							No inline preview yet for {selection.asset_kind}. Use the producer/consumer arrows in
							the graph to navigate. Runs of the upstream script are below.
						</div>
					{/if}
				</Pane>
				<Pane size={45} minSize={20}>
					<AssetRunsPanel
						producers={selectionProducers}
						refreshKey={runsRefreshKey}
						pendingJobId={runsPendingJobId}
						onRunCompleted={() => (previewRefreshKey += 1)}
					/>
				</Pane>
			</Splitpanes>
		{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'flow' && !isDraft}
			<div class="p-3 text-xs text-secondary">
				Flows are not editable inline. Use the open-in-editor button above.
			</div>
		{:else if !isDraft && scriptRes.loading && !script}
			<div class="absolute inset-0 flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={16} class="animate-spin" />
				<span class="text-xs">Loading script…</span>
			</div>
		{:else if !isDraft && scriptRes.error}
			<div class="p-3 text-xs text-red-500">
				Failed to load: {scriptRes.error.message}
			</div>
		{:else if script}
			{#key (script.hash ?? 'draft') + script.language}
				<ScriptEditor
					showCaptures={false}
					noSyncFromGithub
					lang={script.language}
					path={script.path}
					tag={script.tag}
					fixedOverflowWidgets={false}
					previewLayout="bottom"
					customUi={{
						previewPanel: {
							disableHistory: true,
							disableTracing: true,
							disableTriggerCaptures: true,
							disableJsonView: true
						}
					}}
					bind:code={script.content}
					bind:schema={script.schema}
					{args}
				>
					{#snippet editorBarRight()}
						<div>
							<WorkerTagSelect bind:tag={() => script?.tag, (v) => script && (script.tag = v)} />
						</div>
					{/snippet}
				</ScriptEditor>
			{/key}
		{/if}
	</div>
</div>

{#if removeOpen}
	<!-- Single combined modal. Archive is the always-available default;
	     Delete shows only for workspace/super admins because it's
	     irreversible from the UI. Layout mirrors ConfirmationModal so the
	     two confirmation surfaces feel consistent. -->
	<div
		transition:fade={{ duration: 100 }}
		class="fixed top-0 bottom-0 left-0 right-0 z-[9999]"
		role="dialog"
	>
		<div class="fixed inset-0 bg-gray-500 bg-opacity-75"></div>
		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl sm:my-8 sm:w-full sm:max-w-lg sm:p-6"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-800/50"
						>
							<AlertTriangle class="text-red-500 dark:text-red-400" />
						</div>
						<div class="ml-4 flex-1">
							<h3 class="text-lg font-medium text-primary">Archive or delete this script?</h3>
							<div class="mt-2 text-sm text-secondary flex flex-col gap-2">
								<p>
									<span class="font-mono text-xs">{script?.path ?? ''}</span>
								</p>
								<p>
									<span class="font-medium">Archive</span> removes the script from the pipeline graph
									and stops every trigger from firing it (schedule, webhook, asset event, …). History
									is preserved and the script can be unarchived later.
								</p>
								{#if canHardDelete}
									<p>
										<span class="font-medium text-red-700 dark:text-red-400">Delete</span> permanently
										drops every version and draft at this path. Restorable by a workspace admin from
										the trashbin within 3 days, otherwise irrecoverable.
									</p>
								{/if}
							</div>
						</div>
					</div>
					<div class="flex items-center gap-2 flex-row-reverse mt-4">
						<Button
							disabled={removing}
							onclick={archive}
							variant="accent"
							color="red"
							size="sm"
							destructive
						>
							{#if removing}
								<Loader2 class="animate-spin" />
							{/if}
							<span class="min-w-20">Archive</span>
						</Button>
						<Button
							disabled={removing}
							onclick={() => (removeOpen = false)}
							variant="default"
							size="sm"
						>
							Cancel
						</Button>
						{#if canHardDelete}
							<!-- Sits at the far left of the row (flex-row-reverse) to
							     visually separate the irreversible option from the
							     safer Archive. No flex-1 wrapper — the button sizes
							     to its label. -->
							<Button
								disabled={removing}
								onclick={deleteScript}
								variant="contained"
								color="red"
								size="sm"
								destructive
							>
								Delete permanently
							</Button>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
