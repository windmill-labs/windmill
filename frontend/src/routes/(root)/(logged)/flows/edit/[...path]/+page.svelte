<script lang="ts">
	import { FlowService, type Flow, DraftService } from '$lib/gen'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
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
	import { UserDraft } from '$lib/userDraft.svelte'

	let version: undefined | number = $state(undefined)
	let nodraft = page.url.searchParams.get('nodraft')

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
		if (nodraft) {
			let url = new URL(page.url.href)
			url.search = ''
			replaceState(url.toString(), page.state)
		}
	})

	const flowDraftPath = page.params.path ?? ''
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

	let draftTriggersFromUrl: Trigger[] | undefined = $state(undefined)
	let selectedTriggerIndexFromUrl: number | undefined = $state(undefined)
	let loadedFromHistoryFromUrl:
		| { flowJobInitial: boolean | undefined; stepsState: Record<string, stepState> }
		| undefined = $state(undefined)

	let flowBuilder: FlowBuilder | undefined = $state(undefined)
	let notFound = $state(false)
	async function loadFlow(): Promise<void> {
		console.log('loadFlow')
		loading = true
		let flow: Flow
		// Currently there is no way to get version of flow with flow.
		// So we have to request it here
		version = (
			await FlowService.getFlowLatestVersion({
				workspace: $workspaceStore!,
				path: page.params.path ?? ''
			})
		).id

		const flowWithDraft = await FlowService.getFlowByPathWithDraft({
			workspace: $workspaceStore!,
			path: page.params.path ?? ''
		})
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

		if (localDraft != undefined) {
			// Returning visit: local autosave is the source of truth. If it
			// matches the backend's view exactly, drop the toast — otherwise
			// let the user decide between keeping local or discarding.
			const localClean = cleanValueProperties(localDraft)
			const backendClean = cleanValueProperties(backendFlow)
			if (orderedJsonStringify(localClean) === orderedJsonStringify(backendClean)) {
				flow = backendFlow
				flowHandle.draft = backendFlow
			} else {
				flow = localDraft
				sendUserToast('Flow loaded from local autosave', false, [
					{
						label: 'Discard local autosave',
						callback: () => {
							flowHandle.draft = backendFlow
							loadFlow()
						}
					},
					{
						label: 'Show diff',
						callback: async () => {
							diffDrawer?.openDrawer()
							diffDrawer?.setDiff({
								mode: 'simple',
								original: backendClean,
								current: localClean,
								title: `${flowWithDraft.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
								button: {
									text: 'Discard autosave',
									onClick: () => {
										flowHandle.draft = backendFlow
										loadFlow()
									}
								}
							})
						}
					}
				])
			}
		} else {
			flow = backendFlow
			flowHandle.draft = backendFlow
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
			flowBuilder?.setDraftTriggers(undefined)
		}

		await initFlow(flow, flowStore, flowStateStore)
		loading = false
		selectedId = page.url.searchParams.get('selected') ?? 'settings-metadata'
		flowBuilder?.loadFlowState()
	}

	$effect(() => {
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
			// Don't UserDraft.remove here — the route's flowStore reads the
			// handle's draft directly, so clearing it before goto would blank
			// the flow value and trip UnsavedConfirmationModal. The entry's
			// ref count drops on unmount and the next visit's load-time diff
			// will silently overwrite localStorage with the deployed value.
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
