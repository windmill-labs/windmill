<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Edit Flow ${params.path}` }
		}
	}
</script>

<script lang="ts">
	import { FlowService, type Flow } from '$lib/gen'

	import { page } from '$app/stores'
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { workspaceStore } from '$lib/stores'
	import { decodeArgs, decodeState, emptySchema } from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	const initialState = $page.url.searchParams.get('state')
	let stateLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined
	const initialArgs = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	let loading = false

	let selectedId: string = 'settings-graph'

	let flow: Flow = {
		path: $page.params.path,
		summary: '',
		edited_by: '',
		edited_at: '',
		value: { modules: [] },
		archived: false,
		extra_perms: {},
		schema: emptySchema()
	}

	let initialPath: string = ''

	initFlow(flow)

	async function loadFlow(): Promise<void> {
		loading = true
		flow =
			stateLoadedFromUrl != undefined && stateLoadedFromUrl?.flow?.path == flow.path
				? stateLoadedFromUrl.flow
				: await FlowService.getFlowByPath({
						workspace: $workspaceStore!,
						path: flow.path
				  })
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
