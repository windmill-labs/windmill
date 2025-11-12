<script lang="ts">
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	import { workspaceStore } from '$lib/stores'
	import { decodeState } from '$lib/utils'

	let content = localStorage.getItem('svelvet')
	const { modules, failureModule, preprocessorModule, notes } = content
		? decodeState(content)
		: { modules: [], failureModule: undefined, preprocessorModule: undefined }
</script>

<FlowGraphV2
	workspace={$workspaceStore}
	triggerNode={false}
	{modules}
	{failureModule}
	{preprocessorModule}
	{notes}
/>
<a
	download="flow.json"
	href={'data:text/json;charset=utf-8,' +
		encodeURIComponent(
			JSON.stringify(
				{ value: { modules, failureModule, preprocessorModule }, summary: '' },
				null,
				4
			)
		)}>Download</a
>
