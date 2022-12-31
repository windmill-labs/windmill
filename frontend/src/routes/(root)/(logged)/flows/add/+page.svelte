<script lang="ts">
	import { page } from '$app/stores'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { FlowService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { decodeState, emptySchema } from '$lib/utils'

	const hubId = $page.url.searchParams.get('hub')
	const templatePath = $page.url.searchParams.get('template')
	const initialState = hubId || templatePath ? undefined : localStorage.getItem('flow')

	let selectedId: string = 'settings-metadata'
	let loading = false

	async function loadFlow() {
		loading = true
		let state = initialState ? decodeState(initialState) : undefined
		let flow: Flow = state?.flow ?? {
			path: '',
			summary: '',
			value: { modules: [] },
			edited_by: '',
			edited_at: '',
			archived: false,
			extra_perms: {},
			schema: emptySchema()
		}

		state?.selectedId && (selectedId = state?.selectedId)
		if (templatePath) {
			const template = await FlowService.getFlowByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			Object.assign(flow, template)
			const oldPath = flow.path.split('/')
			flow.path = `u/${$userStore?.username}/${oldPath[oldPath.length - 1]}`
			flow = flow
			$page.url.searchParams.delete('template')
			selectedId = 'settings-graph'
		} else if (hubId) {
			const hub = await FlowService.getHubFlowById({ id: Number(hubId) })
			delete hub['comments']
			flow.path = `u/${$userStore?.username}/flow_${hubId}`
			Object.assign(flow, hub.flow)
			flow = flow
			$page.url.searchParams.delete('hub')
			selectedId = 'settings-graph'
		}

		await initFlow(flow)
		loading = false
	}

	loadFlow()

	$dirtyStore = true
</script>

<FlowBuilder {selectedId} {loading} />
