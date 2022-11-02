<script lang="ts">
	import { truncate } from '$lib/utils'
	import Tooltip from './Tooltip.svelte'
	import json from 'svelte-highlight/languages/json'
	import { Highlight } from 'svelte-highlight'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Drawer from './common/drawer/Drawer.svelte'
	import { DrawerContent } from './common'

	export let value: any
	let jsonViewer: Drawer
	let jsonViewerContent: any | undefined

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	async function getResource(path: string) {
		jsonViewerContent = (await ResourceService.getResource({ workspace: $workspaceStore!, path }))
			.value
	}

	$: asJson = JSON.stringify(value, null, 4)
</script>

<Drawer bind:this={jsonViewer} size="400px">
	<DrawerContent title="See JSON" on:close={jsonViewer.toggleDrawer}>
		<Highlight language={json} code={JSON.stringify(jsonViewerContent, null, 4)} />
	</DrawerContent>
</Drawer>

{#if value == '<function call>'}
	{'<function call>'}<Tooltip
		>The arg was none and the default argument of the script is a function call, hence the actual
		value used for this arg was the output of the script's function call for this arg</Tooltip
	>
{:else if isString(value) && value.startsWith('$res:')}
	<button
		class="text-xs text-blue-500"
		on:click={async () => {
			await getResource(value.substring('$res:'.length))
			jsonViewer.toggleDrawer()
		}}>{value}</button
	>{:else if asJson.length > 40}
	{truncate(asJson, 40)}<a
		href="#json"
		on:click|preventDefault={() => {
			jsonViewerContent = value
			jsonViewer.toggleDrawer()
		}}>(+)</a
	>
{:else}
	{asJson}
{/if}
