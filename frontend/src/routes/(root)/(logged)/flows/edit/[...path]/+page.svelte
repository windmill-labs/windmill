<script lang="ts">
	import { FlowService, type Flow } from '$lib/gen'

	import { page } from '$app/stores'
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { workspaceStore } from '$lib/stores'
	import { decodeArgs, decodeState, emptySchema, sendUserToast } from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { goto } from '$app/navigation'
	import { writable } from 'svelte/store'
	import type { FlowState } from '$lib/components/flows/flowState'

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
								goto(`/flows/edit/${flow!.path}`)
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
		selectedId = stateLoadedFromUrl?.selectedId
		$dirtyStore = false
	}

	$: {
		if ($workspaceStore) {
			loadFlow()
		}
	}
</script>

<div id="monaco-widgets-root" class="monaco-editor" style="z-index: 999;" />

<FlowBuilder
	{flowStore}
	{flowStateStore}
	initialPath={$page.params.path}
	{selectedId}
	{initialArgs}
	{loading}
/>
