<script lang="ts">
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	import { decodeState } from '$lib/utils'

	let content = localStorage.getItem('svelvet')
	const { modules, failureModule } = content
		? decodeState(content)
		: { modules: [], failureModule: undefined }
</script>

<FlowGraphV2 {modules} {failureModule} />
<a
	download="flow.json"
	href={'data:text/json;charset=utf-8,' +
		encodeURIComponent(JSON.stringify({ value: { modules, failureModule }, summary: '' }, null, 4))}
	>Download</a
>
