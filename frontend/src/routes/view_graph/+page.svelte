<script lang="ts">
	import FlowGraph from '$lib/components/graph/FlowGraph.svelte'
	import { decodeState } from '$lib/utils'

	let content = localStorage.getItem('svelvet')
	const { modules, failureModule } = content
		? decodeState(content)
		: { modules: [], failureModule: undefined }
</script>

<FlowGraph fullSize {modules} {failureModule} />
<a
	download="flow.json"
	href={'data:text/json;charset=utf-8,' +
		encodeURIComponent(JSON.stringify({ value: { modules, failureModule }, summary: '' }, null, 4))}
	>Download</a
>
