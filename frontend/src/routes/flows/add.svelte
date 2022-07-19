<script lang="ts">
	import { page } from '$app/stores'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import { initFlow } from '$lib/components/flows/flowStore'
	import type { Flow } from '$lib/gen'
	import { emptySchema } from '$lib/utils'

	const initialState = $page.url.searchParams.get('state')

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

	initFlow(flow)
</script>

<FlowBuilder />
