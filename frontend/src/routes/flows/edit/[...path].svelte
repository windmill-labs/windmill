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
	import { decodeState, emptySchema } from '$lib/utils'
	import { initFlow } from '$lib/components/flows/flowStore'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	const initialState = $page.url.searchParams.get('state')
	let flowLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

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
		flow =
			flowLoadedFromUrl != undefined && flowLoadedFromUrl.path == flow.path
				? flowLoadedFromUrl
				: await FlowService.getFlowByPath({
						workspace: $workspaceStore!,
						path: flow.path
				  })
		initialPath = flow.path

		initFlow(flow)
		$dirtyStore = false
	}

	$: {
		if ($workspaceStore) {
			loadFlow()
		}
	}


</script>

<FlowBuilder {initialPath} />
