<script lang="ts">
	import { FlowService, type Flow } from '$lib/gen'

	import { page } from '$app/stores'
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { workspaceStore } from '$lib/stores'
	import { decodeArgs, decodeState, emptySchema, sendUserToast } from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { goto } from '$app/navigation'

	let nodraft = $page.url.searchParams.get('nodraft')
	const initialState = nodraft ? undefined : localStorage.getItem(`flow-${$page.params.path}`)
	let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined
	const initialArgs = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	let loading = false

	let selectedId: string = 'settings-metadata'

	let initialPath: string = ''

	async function loadFlow(): Promise<void> {
		loading = true
		let flow: Flow
		if (stateLoadedFromUrl != undefined && stateLoadedFromUrl?.flow?.path == $page.params.path) {
			sendUserToast('Flow restored from draft')
			flow = stateLoadedFromUrl.flow
		} else {
			flow = await FlowService.getFlowByPath({
				workspace: $workspaceStore!,
				path: $page.params.path
			})
		}
		initialPath = flow.path

		await initFlow(flow)
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

<FlowBuilder {initialPath} {selectedId} {initialArgs} {loading} />
