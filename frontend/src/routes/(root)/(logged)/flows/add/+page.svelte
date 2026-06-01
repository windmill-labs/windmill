<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { importFlowStore, initFlow } from '$lib/components/flows/flowStore.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { initialArgsStore, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { decodeState, emptySchema, type StateStore } from '$lib/utils'
	import { tick } from 'svelte'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { UserDraft } from '$lib/userDraft.svelte'

	// "+ Flow" buttons navigate with ?nodraft=true to signal "start fresh".
	// Wipe the persisted empty-path autosave and strip the flag from the URL
	// synchronously so a reload doesn't wipe the freshly-started draft. A
	// plain reload of /flows/add (no nodraft) instead restores whatever the
	// user was last working on.
	if (page.url.searchParams.get('nodraft') && typeof window !== 'undefined') {
		UserDraft.remove('flow', '')
		const url = new URL(window.location.href)
		url.searchParams.delete('nodraft')
		window.history.replaceState(window.history.state, '', url.toString())
	}

	const hubId = page.url.searchParams.get('hub')
	const templatePath = page.url.searchParams.get('template')
	const templateId = page.url.searchParams.get('template_id')
	const isFork = page.url.searchParams.get('fork')

	let forkState: any = undefined
	if (isFork) {
		const forkJson = localStorage.getItem('fork_flow')
		if (forkJson) {
			try {
				forkState = JSON.parse(forkJson)
			} catch {}
			localStorage.removeItem('fork_flow')
		} else if ((window.opener as any)?.__forkPreviewData) {
			forkState = (window.opener as any).__forkPreviewData
			delete (window.opener as any).__forkPreviewData
		}
	}

	let selectedId: string = $state('settings-metadata')
	let loading = $state(false)

	let initialPath: string | undefined = $state(undefined)
	let pathStoreInit: string | undefined = $state(undefined)

	let initialArgs = $state({})
	if ($initialArgsStore) {
		initialArgs = $initialArgsStore
		$initialArgsStore = undefined
	}
	// initialArgs may also be set from decoded state below (e.g. fork preview)
	let flowBuilder: FlowBuilder | undefined = $state(undefined)

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

	const flowHandle = UserDraft.use<Flow>('flow', '', { defaultValue: emptyFlow() })

	const flowStore: StateStore<Flow> = {
		get val() {
			return flowHandle.draft ?? emptyFlow()
		},
		set val(v: Flow) {
			flowHandle.draft = v
		}
	}
	const flowStateStore = $state({ val: {} })

	let draftTriggersFromUrl: Trigger[] | undefined = $state(undefined)
	let selectedTriggerIndexFromUrl: number | undefined = $state(undefined)
	async function loadFlow() {
		loading = true
		// Start from the persisted autosave, not a fresh `emptyFlow()`. The
		// branches below override `flow` when the user explicitly asked for a
		// different starting point (import/fork/URL state/template/hub); a
		// plain reload of /flows/add (no query params) falls through with
		// the LS value intact so the user's last session is restored.
		let flow: Flow = flowHandle.draft ?? emptyFlow()

		let state = forkState
		const initialStateQuery = page.url.hash != '' ? page.url.hash.slice(1) : undefined

		if (initialStateQuery) {
			state = decodeState(initialStateQuery)
		}
		if ($importFlowStore) {
			flow = $importFlowStore
			$importFlowStore = undefined
			sendUserToast('Flow loaded from YAML/JSON')
		} else if (!templatePath && !hubId && state) {
			flow = state.flow
			pathStoreInit = state.path
			if (state.initialArgs) {
				initialArgs = state.initialArgs
			}
			draftTriggersFromUrl = state.draft_triggers
			selectedTriggerIndexFromUrl = state.selected_trigger
			flowBuilder?.setDraftTriggers(draftTriggersFromUrl)
			flowBuilder?.setSelectedTriggerIndex(selectedTriggerIndexFromUrl)
			state?.selectedId && (selectedId = state?.selectedId)
		} else {
			if (templatePath) {
				let template: Flow
				if (templateId) {
					template = await FlowService.getFlowVersion({
						workspace: $workspaceStore!,
						version: parseInt(templateId)
					})
				} else {
					template = await FlowService.getFlowByPath({
						workspace: $workspaceStore!,
						path: templatePath
					})
				}
				// Template/hub flows are an explicit "start fresh from this
				// content" — drop any previous empty-path autosave and use
				// the freshly built flow as the baseline.
				flow = emptyFlow()
				Object.assign(flow, template)
				const oldPath = templatePath.split('/')
				initialPath = `u/${$userStore?.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')}/${
					oldPath[oldPath.length - 1]
				}_fork`
				goto('?', { replaceState: true })
				selectedId = 'settings-metadata'
			} else if (hubId) {
				const hub = await FlowService.getHubFlowById({ id: Number(hubId) })
				delete hub['comments']
				initialPath = `u/${$userStore?.username
					.split('@')[0]
					.replace(/[^a-zA-Z0-9_]/g, '')}/flow_${hubId}`
				flow = emptyFlow()
				Object.assign(flow, hub.flow)
				if (flow.value.preprocessor_module?.value.type === 'rawscript') {
					flow.value.preprocessor_module.value.content = replaceScriptPlaceholderWithItsValues(
						hubId,
						flow.value.preprocessor_module.value.content
					)
				}
				goto('?', { replaceState: true })
				selectedId = 'constants'
			}
		}
		await initFlow(flow, flowStore, flowStateStore)
		flowBuilder?.loadFlowState()
		loading = false

		// Trigger tutorial after everything is initialized
		const tutorialParam = page.url.searchParams.get('tutorial')
		if (tutorialParam) {
			// Wait for critical elements to be ready before triggering tutorial
			await tick()
			let attempts = 0
			while (attempts < 20 && !document.querySelector('#flow-editor-virtual-Input')) {
				await new Promise((resolve) => setTimeout(resolve, 100))
				attempts++
			}
			flowBuilder?.triggerTutorial()
		}
	}

	loadFlow()
</script>

<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<FlowBuilder
	onSaveInitial={(e) => {
		UserDraft.remove('flow', '')
		if ($workspaceStore) invalidate($workspaceStore, 'flow')
		goto(`/flows/edit/${e.path}?selected=${e.id}`)
	}}
	onDeploy={(e) => {
		UserDraft.remove('flow', '')
		if ($workspaceStore) invalidate($workspaceStore, 'flow')
		goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
	}}
	onDetails={(e) => {
		goto(`/flows/get/${e.path}?workspace=${$workspaceStore}`)
	}}
	onNavigate={(item) => goto(editPathFor(item))}
	{initialPath}
	{pathStoreInit}
	liveEditorDraftStoragePath=""
	bind:this={flowBuilder}
	newFlow
	{initialArgs}
	{flowStore}
	{flowStateStore}
	{selectedId}
	{loading}
	{draftTriggersFromUrl}
	{selectedTriggerIndexFromUrl}
	noInitial
>
	<UnsavedConfirmationModal
		getInitialAndModifiedValues={flowBuilder?.getInitialAndModifiedValues}
	/>
</FlowBuilder>
