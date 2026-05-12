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
	import { afterNavigate, replaceState } from '$app/navigation'

	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { untrack } from 'svelte'
	import type { stepState } from '$lib/components/stepHistoryLoader.svelte'
	import { page } from '$app/state'

	let version: undefined | number = $state(undefined)

	// `initialArgs` is captured once at mount — it's the session's initial
	// argument set. Per-flow autosave (`stateLoadedFromUrl`) and the
	// `nodraft` flag are re-read inside `loadFlow` / the navigation hook so
	// picker navigation doesn't reuse the original path's state.
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

	afterNavigate(() => {
		if (page.url.searchParams.get('nodraft')) {
			let url = new URL(page.url.href)
			url.search = ''
			replaceState(url.toString(), page.state)
		}
	})

	export const flowStore: StateStore<Flow> = $state({
		val: {
			summary: '',
			value: { modules: [] },
			path: '',
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {},
			schema: emptySchema()
		}
	})
	const flowStateStore = $state({ val: {} })

	let loading = $state(false)

	let selectedId: string = $state('settings-metadata')

	let nobackenddraft = false

	// One-shot read of mount-time autosave, used only to seed the initial
	// `savedPrimarySchedule` before `loadFlow` runs. `loadFlow` itself
	// re-reads localStorage on every invocation (see comment there).
	const initialAutosave = (() => {
		if (page.url.searchParams.get('nodraft')) return undefined
		const raw = localStorage.getItem(`flow-${page.params.path}`)
		return raw != undefined ? decodeState(raw) : undefined
	})()
	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(initialAutosave?.primarySchedule)

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

		// Re-read autosave per load. The component doesn't remount when the
		// picker navigates between flows, so capturing at module init would
		// keep reusing the original flow's state.
		const stored = page.url.searchParams.get('nodraft')
			? undefined
			: localStorage.getItem(`flow-${page.params.path}`)
		const stateLoadedFromUrl = stored != undefined ? decodeState(stored) : undefined

		let flow: Flow
		let statePath = stateLoadedFromUrl?.path
		if (stateLoadedFromUrl != undefined && statePath == page.params.path) {
			// Currently there is no way to get version of flow with flow.
			// So we have to request it here
			const v = (
				await FlowService.getFlowLatestVersion({
					workspace: $workspaceStore!,
					path: statePath
				})
			)?.id
			if (tok !== loadFlowToken) return
			version = v

			if (version == undefined) {
				notFound = true
				sendUserToast(`Flow not found at path ${statePath}`, true)
				return
			}

			const sf = await FlowService.getFlowByPathWithDraft({
				workspace: $workspaceStore!,
				path: statePath
			})
			if (tok !== loadFlowToken) return
			savedFlow = sf

			const draftOrDeployed = cleanValueProperties(savedFlow?.draft || savedFlow)
			const urlScript = cleanValueProperties(
				$state.snapshot({
					...stateLoadedFromUrl.flow,
					draft_triggers: stateLoadedFromUrl.draft_triggers
				})
			)
			flow = stateLoadedFromUrl.flow
			draftTriggersFromUrl = stateLoadedFromUrl.draft_triggers
			selectedTriggerIndexFromUrl = stateLoadedFromUrl.selected_trigger
			loadedFromHistoryFromUrl = stateLoadedFromUrl.loadedFromHistory
			flowBuilder?.setDraftTriggers(draftTriggersFromUrl)
			flowBuilder?.setSelectedTriggerIndex(selectedTriggerIndexFromUrl)
			flowBuilder?.setLoadedFromHistory(loadedFromHistoryFromUrl)
			const selectedId = stateLoadedFromUrl?.selectedId ?? 'settings-metadata'
			const reloadAction = () => {
				// Discard the localStorage autosave so the next `loadFlow`
				// (re-)read sees an empty slot and falls through to the
				// fetch branch — otherwise we'd re-enter this branch and
				// loop. Scripts dodge this because their state lives in
				// the URL fragment, which `goto` clears for us.
				try {
					localStorage.removeItem(`flow-${statePath}`)
				} catch (e) {
					console.error('error interacting with local storage', e)
				}
				goto(`/flows/edit/${statePath}?selected=${selectedId}`)
				loadFlow()
			}
			if (orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(urlScript)) {
				reloadAction()
			} else {
				sendUserToast('Flow loaded from browser storage', false, [
					{
						label: 'Discard browser autosave and reload',
						callback: reloadAction
					},
					{
						label: 'Show diff',
						callback: async () => {
							diffDrawer?.openDrawer()
							diffDrawer?.setDiff({
								mode: 'simple',
								original: draftOrDeployed,
								current: urlScript,
								title: `${savedFlow?.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
								button: { text: 'Discard autosave', onClick: reloadAction }
							})
						}
					}
				])
			}
		} else {
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
			if (flowWithDraft.draft != undefined && !nobackenddraft) {
				flow = flowWithDraft.draft
				savedPrimarySchedule = flowWithDraft?.draft?.['primary_schedule']
				flowBuilder?.setPrimarySchedule(savedPrimarySchedule)
				flowBuilder?.setDraftTriggers(flowWithDraft?.draft?.['draft_triggers'])

				if (!flowWithDraft.draft_only) {
					const deployed = cleanValueProperties(flowWithDraft)
					const draft = cleanValueProperties(flow)
					const reloadAction = async () => {
						await DraftService.deleteDraft({
							workspace: $workspaceStore!,
							kind: 'flow',
							path: flow.path
						})
						nobackenddraft = true
						loadFlow()
					}
					sendUserToast('flow loaded from latest saved draft', false, [
						{
							label: 'Discard draft and load from latest deployed version',
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
				flow = flowWithDraft
				flowBuilder?.setDraftTriggers(undefined)
			}
		}

		await initFlow(flow, flowStore, flowStateStore)
		if (tok !== loadFlowToken) return
		loading = false
		selectedId = stateLoadedFromUrl?.selectedId ?? page.url.searchParams.get('selected')
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
		goto(`/flows/edit/${savedFlow.path}`)
		loadFlow()
	}
</script>

<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} isFlow />
{#if notFound}
	<div class="flex flex-col items-center justify-center h-full">
		<h1 class="text-2xl font-bold">Flow not found at path {page.params.path}</h1>
		<p class="text-gray-500">The flow you are looking for does not exist.</p>
	</div>
{:else}
	<FlowBuilder
		onDeploy={(e) => {
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
