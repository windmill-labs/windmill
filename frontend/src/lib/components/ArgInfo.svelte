<script lang="ts">
	import { truncate } from '$lib/utils'
	import Tooltip from './Tooltip.svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Drawer from './common/drawer/Drawer.svelte'
	import { DrawerContent } from './common'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'

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
</script>

<Drawer bind:this={jsonViewer} size="800px">
	<DrawerContent title="Argument Details" on:close={jsonViewer.toggleDrawer}>
		{#if isString(jsonViewerContent)}
			<pre>{jsonViewerContent}</pre>
		{:else}
			<ObjectViewer pureViewer json={jsonViewerContent} />
		{/if}
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
	>
{:else if typeof value !== 'object'}
	{truncate(JSON.stringify(value), 100)}
	{#if JSON.stringify(value).length > 100}
		<button
			class="text-xs text-blue-500"
			on:click={() => {
				jsonViewerContent = value
				jsonViewer.toggleDrawer()
			}}>See expanded</button
		>
	{/if}
{:else}
	<div class="max-h-40 overflow-auto">
		<ObjectViewer collapsed={false} topBrackets={true} pureViewer={true} json={value} />
	</div>
	{#if JSON.stringify(value).length > 120}
		<button
			class="text-xs text-blue-500"
			on:click={() => {
				jsonViewerContent = value
				jsonViewer.toggleDrawer()
			}}>See JSON</button
		>
	{/if}
{/if}
