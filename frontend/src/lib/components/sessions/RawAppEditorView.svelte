<script lang="ts">
	import { untrack } from 'svelte'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import type {
		RawAppRuntimeLogRequester,
		RawAppRunsProvider,
		RawAppScreenshotRequester
	} from '$lib/components/raw_apps/utils'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		isActiveSession = true,
		active = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		/** Forwarded to SessionEditorTarget — only the visible session claims the
		 * workspace's single live-editor slot. */
		isActiveSession?: boolean
		/** Whether this is the visible preview tab (forwarded as isActiveTab). */
		active?: boolean
	} = $props()

	// This tab's own raw-app cell; each open app editor binds its own store.
	const cell = $derived(runtime.rawAppCell(path))
	let diffDrawer: DiffDrawer | undefined = $state()

	// Path typed in the editor header, surfaced when it differs from the stored
	// path. Mirror it into the runtime draft as `draft_path` so the rename
	// mutates this cell's store → the autosave sig changes → the draft is saved
	// (and the home/review/Drafts lists show the friendly name). Mirrors the
	// full-page /apps_raw/edit route.
	let pendingDraftPath = $state<string | undefined>(undefined)
	// The header collapses both "not yet bound" and "reverted to baseline" to
	// `undefined`. `surfacedDraftPath` tells them apart: the initial undefined
	// (before the header binds) must not clobber the `draft_path` seeded by
	// loadRawApp, but a revert/clear after a real typed path must drop the stale
	// friendly name — mirroring the script codec's `else delete draft_path`.
	let surfacedDraftPath = false
	$effect(() => {
		const dp = pendingDraftPath
		untrack(() => {
			const val = cell.store.val
			if (!val) return
			if (dp !== undefined) {
				surfacedDraftPath = true
				if (val.draft_path !== dp) val.draft_path = dp
			} else if (surfacedDraftPath && val.draft_path !== undefined) {
				val.draft_path = undefined
			}
		})
	})

	async function reloadDeployed() {
		await runtime.loadRawApp(workspaceId, path, true, true)
	}

	async function restoreDeployed() {
		diffDrawer?.closeDrawer()
		await runResetToDeployed({
			workspace: workspaceId,
			itemKind: 'raw_app',
			path,
			onResetToDeployed: reloadDeployed
		})
		invalidateWorkspaceDrafts(workspaceId)
	}

	function registerRuntimeLogRequester(requester: RawAppRuntimeLogRequester | undefined) {
		runtime.setRuntimeLogRequester(requester)
	}

	function registerRunsProvider(provider: RawAppRunsProvider | undefined) {
		runtime.setAppRunsProvider(provider)
	}

	function registerScreenshotRequester(requester: RawAppScreenshotRequester | undefined) {
		runtime.setScreenshotRequester(requester)
	}
</script>

{#if cell.saved.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
{/if}
<SessionEditorTarget
	{runtime}
	kind="raw_app"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	isActiveTab={active}
	effectivePath={() => cell.store.val?.path ?? path}
>
	{#snippet editor()}
		{#if cell.store.val}
			<!-- newApp: a draft-only app (no_deployed=true) has a truthy synthesized
			     savedApp but no deployed row, so it must deploy via createApp — keying
			     on !savedApp alone would updateApp a never-deployed path and 404
			     "not found". -->
			<!-- These bind: targets live on this tab's editor cell (cell.store.val /
			     cell.saved.val), reactive state owned by the SessionRuntime class (via
			     rawAppCell), not by a component ancestor — so Svelte's ownership check
			     flags a false positive here. -->
			<!-- svelte-ignore ownership_invalid_binding -->
			<RawAppEditor
				bind:files={cell.store.val.files}
				bind:runnables={cell.store.val.runnables}
				bind:data={cell.store.val.data}
				bind:summary={cell.store.val.summary}
				bind:pendingDraftPath
				newPath={cell.store.val.draft_path ?? cell.store.val.path}
				{path}
				autosaveWorkspace={workspaceId}
				autosavePath={path}
				policy={cell.store.val.policy}
				bind:savedApp={cell.saved.val}
				newApp={!cell.saved.val || cell.saved.val.no_deployed === true}
				{diffDrawer}
				{onNavigate}
				condensedHeader={true}
				onResetToDeployed={reloadDeployed}
				onDeploy={(e) => {
					// Sync the preview to deployed (raw apps deploy only from this editor).
					runtime.syncPreviewWithDeployed(workspaceId, 'raw_app', e.path)
					// Deploying clears the item's pending draft — refresh the Draft Count.
					invalidateWorkspaceDrafts(workspaceId)
				}}
				defaultSidebarCollapsed
				sidebarStorageKey="raw-app-sidebar-collapsed-preview"
				defaultSplitWithPreview={false}
				onRuntimeLogRequester={registerRuntimeLogRequester}
				onRunsProvider={registerRunsProvider}
				onScreenshotRequester={registerScreenshotRequester}
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
