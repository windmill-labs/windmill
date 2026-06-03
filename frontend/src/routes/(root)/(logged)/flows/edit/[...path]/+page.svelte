<script lang="ts">
	import { FlowService, type Flow, DraftService } from '$lib/gen'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import { decodeState, emptySchema, type StateStore } from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore.svelte'
	import { goto } from '$lib/navigation'

	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { tick, untrack } from 'svelte'
	import type { stepState } from '$lib/components/stepHistoryLoader.svelte'
	import { page } from '$app/state'
	import { UserDraft, type UserDraftHandle } from '$lib/userDraft.svelte'

	let version: undefined | number = $state(undefined)

	// `initialArgs` is captured once at mount — it's the session's initial
	// argument set.
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

	// Derived so client-side nav (breadcrumb) re-keys the handle to the new path.
	let flowDraftPath = $derived(page.params.path ?? '')

	// `useMany` keyed off the reactive `flowDraftPath` re-keys the handle on nav;
	// `flowHandle` proxies the current handle so `flowStore` keeps a fixed ref.
	const flowHandles = UserDraft.useMany<Flow>(() => [{ itemKind: 'flow', path: flowDraftPath }])
	const flowHandle: Pick<UserDraftHandle<Flow>, 'draft' | 'setInitial'> = {
		get draft() {
			return flowHandles[0]?.draft
		},
		set draft(value) {
			const handle = flowHandles[0]
			if (handle) handle.draft = value
		},
		setInitial(value) {
			flowHandles[0]?.setInitial(value)
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

	// While `seeding` is true, writes through `flowStore` seed the in-memory
	// draft WITHOUT syncing to the DB — the value was just loaded from the
	// backend, so re-binding it must not write it straight back. Real user edits
	// (after load) flip `seeding` off and go through the syncing setter.
	let seeding = false
	export const flowStore: StateStore<Flow> = {
		get val() {
			return flowHandle.draft ?? emptyFlow()
		},
		set val(v: Flow) {
			if (seeding) {
				flowHandle.setInitial(v)
			} else {
				flowHandle.draft = v
			}
		}
	}
	const flowStateStore = $state({ val: {} })

	let loading = $state(false)

	// Remounts FlowBuilder on nav: false while a reload runs, true once data is
	// ready, so it mounts fresh instead of reusing the previous flow's state.
	let renderEditor = $state(false)

	let selectedId: string = $state('settings-metadata')

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	let draftTriggersFromUrl: Trigger[] | undefined = $state(undefined)
	let selectedTriggerIndexFromUrl: number | undefined = $state(undefined)
	let loadedFromHistoryFromUrl:
		| { flowJobInitial: boolean | undefined; stepsState: Record<string, stepState> }
		| undefined = $state(undefined)

	let flowBuilder: FlowBuilder | undefined = $state(undefined)
	let notFound = $state(false)
	/** Increments per `loadFlow` call. Each in-flight load checks its captured
	 * token against this before writing shared state — if a newer load started,
	 * the older promise no-ops at the next checkpoint. */
	let loadFlowToken = 0
	async function loadFlow(): Promise<void> {
		const tok = ++loadFlowToken
		loading = true
		seeding = true
		let draftTriggersToApply: Trigger[] | undefined = undefined
		let applyPrimarySchedule = false
		// Currently there is no way to get version of flow with flow.
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

		// The editor works off the backend DB draft when present, otherwise the
		// deployed version. Seeding (not assigning) avoids writing the
		// freshly-loaded value straight back to the DB.
		const flow = flowWithDraft.draft != undefined ? flowWithDraft.draft : flowWithDraft
		flowHandle.setInitial(flow)

		if (flowWithDraft.draft != undefined) {
			savedPrimarySchedule = flowWithDraft?.draft?.['primary_schedule']
			applyPrimarySchedule = true
			draftTriggersToApply = flowWithDraft?.draft?.['draft_triggers']
		}

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
		// Loading is done — subsequent edits should sync to the DB.
		seeding = false
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one flow editor
		// to another (e.g. via the workspace picker) reloads the new flow.
		page.params.path
		if ($workspaceStore) {
			untrack(() => {
				renderEditor = false // remount the builder for the navigated-to flow
				loadFlow().catch((e: any) => {
					// A failed load must NOT leave renderEditor stuck false.
					console.error('Failed to load flow', e)
					sendUserToast(`Failed to load flow: ${e?.body ?? e?.message ?? e}`, true)
					seeding = false
					renderEditor = true
				})
			})
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDraft() {
		if (!savedFlow || !savedFlow.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Re-seed from the backend draft (drops the in-memory edits).
		goto(`/flows/edit/${savedFlow.draft.path}`)
		loadFlow()
	}

	async function restoreDeployed() {
		if (!savedFlow) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Explicit user action: delete the DB draft synchronously (don't rely on
		// the debounced autosync) before reloading the deployed version.
		if (savedFlow.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'flow',
				path: savedFlow.path
			})
		}
		UserDraft.remove('flow', flowDraftPath)
		goto(`/flows/edit/${savedFlow.path}`)
		loadFlow()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} isFlow />
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
