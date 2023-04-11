<script lang="ts">
	import FlowGraph from '$lib/components/graph/FlowGraph.svelte'
	import { decodeState } from '$lib/utils'

	let content = localStorage.getItem('svelvet')
	const { modules, failureModule } = content
		? decodeState(content)
		: { modules: [], failureModule: undefined }
</script>

<FlowGraph fullSize {modules} {failureModule} />
