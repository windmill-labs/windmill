<script lang="ts">
	import { FlowService, type Flow, DraftService } from '$lib/gen'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { initialArgsStore, workspaceStore } from '$lib/stores'
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
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { untrack } from 'svelte'
	import type { stepState } from '$lib/components/stepHistoryLoader.svelte'
	import { page } from '$app/state'
	import { UserDraft, checkStaleness, type UserDraftMeta } from '$lib/userDraft.svelte'
	import { notifyRestoredFromLocal } from '$lib/userDraftToast'

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

	let savedFlow:
		| (Flow & {
				draft?: Flow | undefined
		  })
		| undefined = $state(undefined)

	const flowDraftPath = page.params.path ?? ''

	// `?nodraft=true` is the callers' way of saying "skip the local autosave
	// on this load." Wipe the UserDraft entry and strip the flag from the
	// URL synchronously, before the handle is created — same pattern as
	// /flows/add. A plain reload (no nodraft) restores normally.
	if (page.url.searchParams.get('nodraft') && typeof window !== 'undefined') {
		UserDraft.remove('flow', flowDraftPath)
		const url = new URL(window.location.href)
		url.searchParams.delete('nodraft')
		window.history.replaceState(window.history.state, '', url.toString())
	}

	const flowHandle = UserDraft.use<Flow>('flow', flowDraftPath)

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

	let selectedId: string = $state('settings-metadata')

	let nobackenddraft = false

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
	async function loadFlow(): Promise<void> {
		const tok = ++loadFlowToken
		loading = true
		let flow: Flow
		// Currently there is no way to get version of flow with flow.
		// So we have to request it here
		const v = (
			await FlowService.getFlowLatestVersion({
				workspace: $workspaceStore!,
				path: page.params.path ?? ''
			})
		).id
		if (tok !== loadFlowToken) return
		version = v

		const flowWithDraft = await FlowService.getFlowByPathWithDraft({
			workspace: $workspaceStore!,
			path: page.params.path ?? ''
		})
		if (tok !== loadFlowToken) return
		savedFlow = {
			...structuredClone($state.snapshot(flowWithDraft)),
			draft: flowWithDraft.draft
				? {
						...structuredClone($state.snapshot(flowWithDraft.draft)),
						path: flowWithDraft.draft.path ?? flowWithDraft.path // backward compatibility for old drafts missing path
					}
				: undefined
		} as Flow & {
			draft?: Flow & {
				draft_triggers?: Trigger[]
			}
		}

		const backendFlow =
			flowWithDraft.draft != undefined && !nobackenddraft ? flowWithDraft.draft : flowWithDraft
		const localDraft = flowHandle.draft
		const previousMeta = flowHandle.meta
		const newRevs: UserDraftMeta = {
			remoteRev: v,
			remoteDraftRev: flowWithDraft.draft_created_at
		}

		if (localDraft != undefined) {
			const localClean = cleanValueProperties(localDraft)
			const backendClean = cleanValueProperties(backendFlow)
			if (orderedJsonStringify(localClean) === orderedJsonStringify(backendClean)) {
				// Local matches backend exactly — silently drop the autosave.
				flow = backendFlow
				UserDraft.remove('flow', flowDraftPath)
				flowHandle.setDraftAndMeta(backendFlow, newRevs)
			} else {
				flow = localDraft
				const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
				if (cause) {
					pendingBaseline = { baseline: backendFlow, revs: newRevs }
					staleModalCause = cause
					staleModalOpen = true
				} else {
					if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
						// Legacy entry — backfill meta so the next load can detect staleness.
						flowHandle.setMeta(newRevs, { force: true })
					}
					const flowPath = backendFlow.path
					const hasBackendDraft = flowWithDraft.draft != undefined
					notifyRestoredFromLocal(hasBackendDraft, !flowWithDraft.draft_only, {
						onResetToSavedDraft: () => {
							UserDraft.remove('flow', flowDraftPath)
							flowHandle.setDraftAndMeta(backendFlow, newRevs)
							loadFlow()
						},
						onResetToDeployed: async () => {
							if (hasBackendDraft) {
								await DraftService.deleteDraft({
									workspace: $workspaceStore!,
									kind: 'flow',
									path: flowPath
								})
							}
							UserDraft.remove('flow', flowDraftPath)
							// UserDraft.remove only clears localStorage. Drop the
							// entry's in-memory state too so loadFlow doesn't re-read
							// the stale autosave and re-fire the same toast.
							flowHandle.setDraftAndMeta(undefined, {})
							nobackenddraft = true
							loadFlow()
						}
					})
				}
			}
		} else {
			flow = backendFlow
			flowHandle.setDraftAndMeta(backendFlow, newRevs)
		}

		if (flowWithDraft.draft != undefined && !nobackenddraft) {
			savedPrimarySchedule = flowWithDraft?.draft?.['primary_schedule']
			flowBuilder?.setPrimarySchedule(savedPrimarySchedule)
			flowBuilder?.setDraftTriggers(flowWithDraft?.draft?.['draft_triggers'])

			if (!flowWithDraft.draft_only && localDraft == undefined) {
				const deployed = cleanValueProperties(flowWithDraft)
				const draft = cleanValueProperties(flow)
				const reloadAction = async () => {
					await DraftService.deleteDraft({
						workspace: $workspaceStore!,
						kind: 'flow',
						path: flow.path
					})
					UserDraft.remove('flow', flowDraftPath)
					// UserDraft.remove only clears localStorage. The
					// flowHandle's in-memory state still holds the now-
					// deleted DB draft + its meta — loadFlow would treat it
					// as a local autosave and the staleness check would fire
					// a spurious "newer version was deployed" modal because
					// remoteDraftRev moved from "defined" to "undefined".
					// Drop the in-memory state first.
					flowHandle.setDraftAndMeta(undefined, {})
					nobackenddraft = true
					loadFlow()
				}
				sendUserToast('flow loaded from latest saved draft', false, [
					{
						label: 'Reset to deployed',
						callback: reloadAction
					},
					{
						label: 'Show diff',
						callback: async () => {
							diffDrawer?.openDrawer()
							diffDrawer?.setDiff({
								mode: 'simple',
								original: deployed,
								current: draft,
								title: 'Deployed <> Draft',
								button: { text: 'Discard draft', onClick: reloadAction }
							})
						}
					}
				])
			}
		} else {
			flowBuilder?.setDraftTriggers(undefined)
		}

		await initFlow(flow, flowStore, flowStateStore)
		if (tok !== loadFlowToken) return
		loading = false
		selectedId = page.url.searchParams.get('selected') ?? 'settings-metadata'
		flowBuilder?.loadFlowState()
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one flow editor
		// to another (e.g. via the workspace picker) reloads the new flow.
		page.params.path
		if ($workspaceStore) {
			untrack(() => loadFlow())
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDraft() {
		if (!savedFlow || !savedFlow.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('flow', flowDraftPath)
		// Drop the in-memory handle state so loadFlow sees no local draft
		// on the next pass — otherwise the staleness check would compare
		// the stale in-memory meta against the freshly fetched backend and
		// fire a spurious modal.
		flowHandle.setDraftAndMeta(undefined, {})
		goto(`/flows/edit/${savedFlow.draft.path}`)
		loadFlow()
	}

	async function restoreDeployed() {
		if (!savedFlow) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		if (savedFlow.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'flow',
				path: savedFlow.path
			})
		}
		UserDraft.remove('flow', flowDraftPath)
		flowHandle.setDraftAndMeta(undefined, {})
		goto(`/flows/edit/${savedFlow.path}`)
		loadFlow()
	}
</script>

<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} isFlow />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>
{#if notFound}
	<div class="flex flex-col items-center justify-center h-full">
		<h1 class="text-2xl font-bold">Flow not found at path {page.params.path}</h1>
		<p class="text-gray-500">The flow you are looking for does not exist.</p>
	</div>
{:else}
	<FlowBuilder
		onDeploy={(e) => {
			UserDraft.remove('flow', flowDraftPath)
			if ($workspaceStore) invalidate($workspaceStore, 'flow')
			goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onDetails={(e) => {
			goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onSaveDraftOnlyAtNewPath={(e) => {
			goto(`/flows/edit/${e.path}?selected=${e.selectedId}`)
		}}
		onHistoryRestore={() => {
			loadFlow()
		}}
		onNavigate={(item) => goto(editPathFor(item))}
		{flowStore}
		{flowStateStore}
		initialPath={page.params.path ?? ''}
		liveEditorDraftStoragePath={flowDraftPath}
		newFlow={false}
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
	>
		<UnsavedConfirmationModal
			{diffDrawer}
			getInitialAndModifiedValues={flowBuilder?.getInitialAndModifiedValues}
		/>
	</FlowBuilder>
{/if}
