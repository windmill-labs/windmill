<script lang="ts">
	import { FlowService, type Flow } from '$lib/gen'

	import { page } from '$app/stores'
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { workspaceStore } from '$lib/stores'
	import { decodeArgs, decodeState, emptySchema } from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { goto } from '$app/navigation'
	import { writable } from 'svelte/store'
	import type { FlowState } from '$lib/components/flows/flowState'
	import { sendUserToast } from '$lib/toast'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'

	let nodraft = $page.url.searchParams.get('nodraft')
	const initialState = nodraft ? undefined : localStorage.getItem(`flow-${$page.params.path}`)
	let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined
	const initialArgs = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	export const flowStore = writable<Flow>({
		summary: '',
		value: { modules: [] },
		path: '',
		edited_at: '',
		edited_by: '',
		archived: false,
		extra_perms: {},
		schema: emptySchema()
	})
	const flowStateStore = writable<FlowState>({})

	let loading = false

	let selectedId: string = 'settings-metadata'

	let nobackenddraft = false
	async function loadFlow(): Promise<void> {
		loading = true
		let flow: Flow
		if (stateLoadedFromUrl != undefined && stateLoadedFromUrl?.flow?.path == $page.params.path) {
			sendUserToast('Flow restored from ephemeral autosave', false, [
				{
					label: 'Discard autosave and reload',
					callback: () => {
						stateLoadedFromUrl = undefined
						goto(`/flows/edit/${flow!.path}`)
						loadFlow()
					}
				}
			])
			flow = stateLoadedFromUrl.flow
		} else {
			const flowWithDraft = await FlowService.getFlowByPathWithDraft({
				workspace: $workspaceStore!,
				path: $page.params.path
			})
			if (flowWithDraft.draft != undefined && !nobackenddraft) {
				flow = flowWithDraft.draft
				if (!flowWithDraft.draft_only) {
					sendUserToast('flow loaded from latest saved draft', false, [
						{
							label: 'Ignore draft and load from latest deployed version',
							callback: () => {
								stateLoadedFromUrl = undefined
								nobackenddraft = true
								loadFlow()
							}
						}
					])
				}
			} else {
				flow = flowWithDraft
			}
		}

		await initFlow(flow, flowStore, flowStateStore)
		loading = false
		selectedId = stateLoadedFromUrl?.selectedId ?? $page.url.searchParams.get('selected')
		$dirtyStore = false
	}

	$: {
		if ($workspaceStore) {
			loadFlow()
		}
	}
</script>

<div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" />

<UnsavedConfirmationModal />

<FlowBuilder
	on:deploy={() => {
		goto(`/flows/get/${$flowStore.path}?workspace=${$workspaceStore}`)
	}}
	on:details={() => {
		goto(`/flows/get/${$flowStore.path}?workspace=${$workspaceStore}`)
	}}
	{flowStore}
	{flowStateStore}
	initialPath={$page.params.path}
	{selectedId}
	{initialArgs}
	{loading}
/>
