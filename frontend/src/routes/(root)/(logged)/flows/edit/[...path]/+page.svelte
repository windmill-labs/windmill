<script lang="ts">
	import { FlowService, type Flow } from '$lib/gen'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { initialArgsStore, userStore, workspaceStore } from '$lib/stores'
	import {
		cleanValueProperties,
		decodeState,
		emptySchema,
		orderedJsonStringify,
		type StateStore
	} from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore.svelte'
	import { goto } from '$lib/navigation'

	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import DraftSyncConflictModal from '$lib/components/common/confirmationModal/DraftSyncConflictModal.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import OtherUsersDraftsModal, {
		type OtherDraftUser
	} from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { tick, untrack } from 'svelte'
	import type { stepState } from '$lib/components/stepHistoryLoader.svelte'
	import { page } from '$app/state'
	import {
		UserDraft,
		checkStaleness,
		type UserDraftMeta,
		type UserDraftHandle
	} from '$lib/userDraft.svelte'
	import { notifyDraftLoaded, notifyRestoredFromLocal } from '$lib/userDraftToast'

	let version: undefined | number = $state(undefined)

	// `initialArgs` is captured once at mount — it's the session's initial
	// argument set. The flow draft itself lives in UserDraft and is re-read
	// per loadFlow() so picker navigation doesn't reuse the original path's
	// state.
	const urlArgs = page.url.searchParams.get('initial_args')

	let initialArgs = $state({})
	if (urlArgs) {
		initialArgs = decodeState(urlArgs)
	} else if ($initialArgsStore) {
		initialArgs = $initialArgsStore
		$initialArgsStore = undefined
	}

	let savedFlow: Flow | undefined = $state(undefined)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	// `initialPath` is the editor-displayed path. Defaults to the URL
	// path so a deployed (or existing-draft) reload mounts FlowBuilder
	// with the real path; cleared to '' inside the `new_draft` branch
	// below so the Path widget's `initPath` calls `reset()` and seeds
	// the friendly `<random_adj>_flow` name on `/flows/add`.
	let flowInitialPath = $state(page.params.path ?? '')

	// Derived so client-side nav (breadcrumb) re-keys the handle to the new path.
	let flowDraftPath = $derived(page.params.path ?? '')

	/** True when no deployed flow exists at the URL path — either we
	 * landed via `/flows/add → ?new_draft=true` (no fetch yet) or the
	 * fetch came back with `no_deployed`. Controls FlowBuilder's deploy
	 * flow: `createFlow` vs `updateFlow`. Flips back to false once a
	 * deploy lands at this path. */
	let isNewFlow = $state(false)

	// `useMany` keyed off the reactive `flowDraftPath` re-keys the handle on nav;
	// `flowHandle` proxies the current handle so `flowStore` keeps a fixed ref.
	const flowHandles = UserDraft.useMany<Flow>(() => [{ itemKind: 'flow', path: flowDraftPath }])
	const flowHandle: UserDraftHandle<Flow> = {
		get draft() {
			return flowHandles[0]?.draft
		},
		set draft(value) {
			const handle = flowHandles[0]
			if (handle) handle.draft = value
		},
		get meta() {
			return flowHandles[0]?.meta ?? {}
		},
		setDraftAndMeta(value, meta) {
			flowHandles[0]?.setDraftAndMeta(value, meta)
		},
		setMeta(meta, opts) {
			flowHandles[0]?.setMeta(meta, opts)
		}
	}

	function emptyFlow(): Flow {
		return {
			summary: '',
			value: { modules: [] },
			path: '',
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {},
			schema: emptySchema()
		}
	}

	export const flowStore: StateStore<Flow> = {
		get val() {
			return flowHandle.draft ?? emptyFlow()
		},
		set val(v: Flow) {
			flowHandle.draft = v
		}
	}
	const flowStateStore = $state({ val: {} })

	let loading = $state(false)

	// Remounts FlowBuilder on nav: false while a reload runs, true once data is
	// ready, so it mounts fresh instead of reusing the previous flow's state.
	let renderEditor = $state(false)

	let selectedId: string = $state('settings-metadata')

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	// Local-draft staleness modal: opened when the remote has moved on since
	// the local autosave was written.
	let staleModalOpen = $state(false)
	let staleModalCause = $state<'draft' | 'version'>('version')
	let pendingBaseline: { baseline: Flow; revs: UserDraftMeta } | undefined = undefined

	function onStaleLoadLatest(): void {
		if (!pendingBaseline) {
			staleModalOpen = false
			return
		}
		const { baseline, revs } = pendingBaseline
		UserDraft.remove('flow', flowDraftPath)
		flowHandle.setDraftAndMeta(baseline, revs)
		pendingBaseline = undefined
		staleModalOpen = false
		loadFlow()
	}

	function onStaleKeepDraft(): void {
		if (pendingBaseline) {
			flowHandle.setMeta(pendingBaseline.revs, { force: true })
		}
		pendingBaseline = undefined
		staleModalOpen = false
	}

	let draftTriggersFromUrl: Trigger[] | undefined = $state(undefined)
	let selectedTriggerIndexFromUrl: number | undefined = $state(undefined)
	let loadedFromHistoryFromUrl:
		| { flowJobInitial: boolean | undefined; stepsState: Record<string, stepState> }
		| undefined = $state(undefined)

	let flowBuilder: FlowBuilder | undefined = $state(undefined)
	let notFound = $state(false)
	/** Increments per `loadFlow` call. Each in-flight load checks its captured
	 * token against this before writing shared state — if a newer load started
	 * (e.g. picker navigation while a draft-discard reload is in flight),
	 * the older promise no-ops at the next checkpoint. */
	let loadFlowToken = 0
	async function loadFlow(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadFlowToken
		loading = true
		let flow: Flow
		// Builder-dependent setup is captured here and applied AFTER the builder
		// remounts (see end of loadFlow): during a reload renderEditor is false,
		// so flowBuilder is unmounted and direct calls would no-op.
		let draftTriggersToApply: Trigger[] | undefined = undefined
		let applyPrimarySchedule = false
		// `?new_draft=true` (set by `/flows/add`'s redirect) means we
		// landed on a fresh `u/{user}/draft_{uuid}` path that's never
		// been saved. Skip both the latest-version and the get-by-path
		// fetches (they would 404), seed an empty Flow with `path = ''`
		// — the `Path` widget's `initPath` calls `reset()` only when
		// both `path` and `initialPath` are empty, and that's what
		// generates the friendly `<random_adj>_flow` name. Anything
		// non-empty (even `u/{user}/`) is parsed verbatim and the
		// friendly seed never fires.
		if (page.url.searchParams.get('new_draft') === 'true') {
			// No deployed row at the autogenerated `draft_{uuid}` URL
			// path — FlowBuilder's Deploy button must `createFlow` at
			// the user-typed path, not `updateFlow` at the URL.
			isNewFlow = true
			// Clear `initialPath` so the Path widget's `initPath` falls
			// into the empty-path branch and calls `reset()` to seed a
			// friendly auto-name (mirrors the script editor — without
			// this the topbar would just show the raw `draft_{uuid}`).
			flowInitialPath = ''
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			const empty: Flow = {
				path: '',
				summary: '',
				description: '',
				value: { modules: [] },
				schema: {},
				extra_perms: {},
				edited_at: new Date().toISOString(),
				edited_by: ''
			} as unknown as Flow
			savedFlow = structuredClone(empty)
			flowHandle.setDraftAndMeta(empty, {})
			flow = empty
			await initFlow(flow, flowStore, flowStateStore)
			if (tok !== loadFlowToken) return
			loading = false
			selectedId = page.url.searchParams.get('selected') ?? 'settings-metadata'
			renderEditor = true
			return
		}
		// Currently there is no way to get version of flow with flow.
		// So we have to request it here.
		//
		// Tolerate a missing version: when the path is draft-only (no
		// deployed flow at this path), `getFlowLatestVersion` returns no
		// row and `.id` would throw on `null`. The `getFlowByPath` call
		// below still returns the draft via the `get_draft` overlay, so
		// the editor can mount with `version = undefined`.
		let v: number | undefined
		try {
			v = (
				await FlowService.getFlowLatestVersion({
					workspace: $workspaceStore!,
					path: page.params.path ?? ''
				})
			)?.id
		} catch {
			v = undefined
		}
		if (tok !== loadFlowToken) return
		version = v

		const backendFlow = await FlowService.getFlowByPath({
			workspace: $workspaceStore!,
			path: page.params.path ?? '',
			getDraft
		})
		if (tok !== loadFlowToken) return
		otherDraftsUsers = ((backendFlow as any).other_drafts_users ?? []) as OtherDraftUser[]
		if ($workspaceStore && flowDraftPath) {
			UserDraftDbSyncer.recordRemoteSync(
				{ workspace: $workspaceStore, itemKind: 'flow', path: flowDraftPath },
				(backendFlow as any).draft_saved_at as string | undefined
			)
		}
		// Re-evaluate the "new flow" signal on each load — flips to
		// true for draft-only paths and back to false once a deploy
		// lands at this URL path.
		isNewFlow = !!backendFlow.no_deployed
		if (backendFlow.is_draft) {
			notifyDraftLoaded({
				workspace: $workspaceStore!,
				itemKind: 'flow',
				path: page.params.path ?? '',
				draftOnly: backendFlow.no_deployed,
				onResetToDeployed: async () => {
					flowHandle.setDraftAndMeta(undefined, {})
					await loadFlow({ getDraft: false })
				}
			})
		}
		// `backendFlow` is the deployed payload; the user's saved draft
		// (if any) is attached as `.draft`. Layer the draft over the
		// deployed at the field level so downstream code sees the
		// "draft-on-deployed" shape it used to get pre-merged from the
		// server. See /scripts/edit's loader for the rationale.
		const { draft: draftFromBackend, ...deployedFlow } = backendFlow as any
		const effectiveFlow: Flow = draftFromBackend
			? ({ ...deployedFlow, ...draftFromBackend } as Flow)
			: (deployedFlow as Flow)
		savedFlow = structuredClone($state.snapshot(effectiveFlow)) as Flow

		const localDraft = flowHandle.draft
		const previousMeta = flowHandle.meta
		const newRevs: UserDraftMeta = {
			remoteRev: v
		}

		if (localDraft != undefined) {
			const localClean = cleanValueProperties(localDraft)
			const backendClean = cleanValueProperties(effectiveFlow)
			if (orderedJsonStringify(localClean) === orderedJsonStringify(backendClean)) {
				// Local matches backend exactly — silently drop the autosave.
				flow = effectiveFlow
				UserDraft.remove('flow', flowDraftPath)
				flowHandle.setDraftAndMeta(effectiveFlow, newRevs)
			} else {
				flow = localDraft
				const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
				if (cause) {
					pendingBaseline = { baseline: effectiveFlow, revs: newRevs }
					staleModalCause = cause
					staleModalOpen = true
				} else {
					if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
						// Legacy entry — backfill meta so the next load can detect staleness.
						flowHandle.setMeta(newRevs, { force: true })
					}
					notifyRestoredFromLocal(false, true, {
						onResetToSavedDraft: () => {
							UserDraft.remove('flow', flowDraftPath)
							flowHandle.setDraftAndMeta(effectiveFlow, newRevs)
							loadFlow()
						},
						onResetToDeployed: async () => {
							UserDraft.remove('flow', flowDraftPath)
							// UserDraft.remove only clears localStorage. Drop the
							// entry's in-memory state too so loadFlow doesn't re-read
							// the stale autosave and re-fire the same toast.
							flowHandle.setDraftAndMeta(undefined, {})
							loadFlow()
						}
					})
				}
			}
		} else {
			flow = effectiveFlow
			flowHandle.setDraftAndMeta(effectiveFlow, newRevs)
		}

		flowBuilder?.setDraftTriggers(undefined)

		await initFlow(flow, flowStore, flowStateStore)
		if (tok !== loadFlowToken) return
		loading = false
		selectedId = page.url.searchParams.get('selected') ?? 'settings-metadata'
		// Remount the builder first, then apply builder-dependent setup once it
		// has mounted — otherwise (during a reload) these would no-op on the
		// unmounted builder and editor state restoration would be skipped.
		renderEditor = true
		await tick()
		if (tok !== loadFlowToken) return
		if (applyPrimarySchedule) flowBuilder?.setPrimarySchedule(savedPrimarySchedule)
		flowBuilder?.setDraftTriggers(draftTriggersToApply)
		flowBuilder?.loadFlowState()
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one flow editor
		// to another (e.g. via the workspace picker) reloads the new flow.
		page.params.path
		if ($workspaceStore) {
			untrack(() => {
				renderEditor = false // remount the builder for the navigated-to flow
				loadFlow().catch((e: any) => {
					// A failed load must NOT leave renderEditor stuck false — otherwise
					// the editor pane disappears and never remounts. Surface the error
					// and remount so the user isn't stranded on a blank pane.
					console.error('Failed to load flow', e)
					sendUserToast(`Failed to load flow: ${e?.body ?? e?.message ?? e}`, true)
					renderEditor = true
				})
			})
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDeployed() {
		if (!savedFlow) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('flow', flowDraftPath)
		flowHandle.setDraftAndMeta(undefined, {})
		goto(`/flows/edit/${savedFlow.path}`)
		loadFlow()
	}
</script>

<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} isFlow />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>
{#if $workspaceStore && flowDraftPath}
	<DraftSyncConflictModal
		query={{ workspace: $workspaceStore, itemKind: 'flow', path: flowDraftPath }}
		onLoadFromServer={() => loadFlow()}
		getLocalDraft={() => flowHandle.draft}
	/>
{/if}
{#if $workspaceStore && flowDraftPath && otherDraftsUsers.length > 0}
	{#key flowDraftPath}
		<OtherUsersDraftsModal
			workspace={$workspaceStore}
			itemKind="flow"
			path={flowDraftPath}
			currentUserUsername={$userStore?.username}
			{otherDraftsUsers}
			editPathFor={(forkedPath) => `/flows/edit/${forkedPath}`}
		/>
	{/key}
{/if}
{#if notFound}
	<div class="flex flex-col items-center justify-center h-full">
		<h1 class="text-2xl font-bold">Flow not found at path {page.params.path}</h1>
		<p class="text-gray-500">The flow you are looking for does not exist.</p>
	</div>
{:else if renderEditor}
	<FlowBuilder
		onDeploy={(e) => {
			UserDraft.remove('flow', flowDraftPath)
			if ($workspaceStore) invalidate($workspaceStore, 'flow')
			goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onDetails={(e) => {
			goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onHistoryRestore={() => {
			loadFlow()
		}}
		onNavigate={(item) => goto(editPathFor(item))}
		{flowStore}
		{flowStateStore}
		bind:initialPath={flowInitialPath}
		liveEditorDraftStoragePath={flowDraftPath}
		newFlow={isNewFlow}
		{selectedId}
		{initialArgs}
		{loading}
		bind:this={flowBuilder}
		bind:savedFlow
		{diffDrawer}
		{savedPrimarySchedule}
		{draftTriggersFromUrl}
		{selectedTriggerIndexFromUrl}
		{version}
		{loadedFromHistoryFromUrl}
	/>
{/if}
