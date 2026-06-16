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
		RawAppRunsProvider
	} from '$lib/components/raw_apps/utils'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		isActiveSession = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		/** Forwarded to SessionEditorTarget — only the visible session claims the
		 * workspace's single live-editor slot. */
		isActiveSession?: boolean
	} = $props()

	let diffDrawer: DiffDrawer | undefined = $state()

	// Path typed in the editor header, surfaced when it differs from the stored
	// path. Mirror it into the runtime draft as `draft_path` so the rename
	// mutates runtime.rawApp.val → the autosave sig changes → the draft is saved
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
			const val = runtime.rawApp.val
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
</script>

{#if runtime.savedRawApp.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
{/if}
<SessionEditorTarget
	{runtime}
	kind="raw_app"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	effectivePath={() => runtime.rawApp.val?.path ?? path}
>
	{#snippet editor()}
		{#if runtime.rawApp.val}
			<!-- newApp: a draft-only app (no_deployed=true) has a truthy synthesized
			     savedApp but no deployed row, so it must deploy via createApp — keying
			     on !savedApp alone would updateApp a never-deployed path and 404
			     "not found". -->
			<RawAppEditor
				bind:files={runtime.rawApp.val.files}
				bind:runnables={runtime.rawApp.val.runnables}
				bind:data={runtime.rawApp.val.data}
				bind:summary={runtime.rawApp.val.summary}
				bind:pendingDraftPath
				newPath={runtime.rawApp.val.draft_path ?? runtime.rawApp.val.path}
				{path}
				autosaveWorkspace={workspaceId}
				autosavePath={path}
				policy={runtime.rawApp.val.policy}
				bind:savedApp={runtime.savedRawApp.val}
				newApp={!runtime.savedRawApp.val || runtime.savedRawApp.val.no_deployed === true}
				{diffDrawer}
				{onNavigate}
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
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
