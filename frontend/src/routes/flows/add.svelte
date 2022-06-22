<script lang="ts">
	import { page } from '$app/stores'

	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import type { Flow } from '../../gen'
	import { emptySchema } from '../../utils'

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
</script>

<FlowBuilder {flow} />
