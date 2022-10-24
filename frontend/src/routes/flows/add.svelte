<script context="module">
	export function load() {
		return {
			stuff: { title: `New Flow` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { decodeState, emptySchema, sendUserToast } from '$lib/utils'

	const initialState = $page.url.searchParams.get('state')
	const hubId = $page.url.searchParams.get('hub')

	const templatePath = $page.url.searchParams.get('template')

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

		if (templatePath) {
			const template = await FlowService.getFlowByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			Object.assign(flow, template)
			flow = flow
			$page.url.searchParams.delete('template')
			sendUserToast('Code & arguments have been loaded from template.')
		} else if (hubId) {
			const hub = (await FlowService.getHubFlowById({ id: Number(hubId) })).flow
			Object.assign(flow, hub)
			flow = flow
			$page.url.searchParams.delete('hub')
			sendUserToast(`Flow has been loaded from hub flow id ${hubId}.`)
		}
		initFlow(flow)
	}

	loadFlow()

	$dirtyStore = true
</script>

<FlowBuilder />
