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
	import { initFlow } from '$lib/components/flows/flowStore'
	import { FlowService, type Flow } from '$lib/gen'
	import { emptySchema, sendUserToast } from '$lib/utils'

	const initialState = $page.url.searchParams.get('state')
	const hubId = $page.url.searchParams.get('hub')

	let flow: Flow =
		initialState != undefined
			? JSON.parse(atob(initialState))
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

	async function loadFlow() {
		if (hubId) {
			const hub = (await FlowService.getHubFlowById({ id: Number(hubId) })).flow
			flow.summary = hub?.summary ?? ''
			flow.value = hub?.value ?? { modules: [] }
			flow.description = hub?.description
			flow.schema = hub?.schema ?? emptySchema()
			flow = flow
			$page.url.searchParams.delete('hub')
			sendUserToast(`Flow has been loaded from hub flow id ${hubId}.`)
		}
		initFlow(flow)
	}

	loadFlow()
</script>

<FlowBuilder />
