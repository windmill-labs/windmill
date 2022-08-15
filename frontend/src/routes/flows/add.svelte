<script context="module">
	export function load() {
		return {
			stuff: { title: `New Flow` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { initFlowState } from '$lib/components/flows/flowState'
	import { initFlow, mode } from '$lib/components/flows/flowStore'
	import { FlowService, type Flow } from '$lib/gen'
	import { decodeState, emptySchema, sendUserToast } from '$lib/utils'

	const initialState = $page.url.searchParams.get('state')
	const hubId = $page.url.searchParams.get('hub')

	async function loadFlow() {
		let flow: Flow =
			initialState != undefined
				? decodeState(initialState)
				: {
						path: '',
						summary: '',
						value: { modules: [] },
						edited_by: '',
						edited_at: '',
						archived: false,
						extra_perms: {},
						schema: emptySchema()
				  }

		if (hubId) {
			const hub = (await FlowService.getHubFlowById({ id: Number(hubId) })).flow
			Object.assign(flow, hub)
			flow = flow
			$page.url.searchParams.delete('hub')
			sendUserToast(`Flow has been loaded from hub flow id ${hubId}.`)
		}
		$mode = 'push'
		initFlowState(flow)
		initFlow(flow)
	}

	loadFlow()
</script>

<FlowBuilder />
