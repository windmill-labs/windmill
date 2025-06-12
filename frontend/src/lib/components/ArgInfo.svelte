<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { copyToClipboard, truncate } from '$lib/utils'
	import { ClipboardCopy, Expand } from 'lucide-svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button, DrawerContent } from './common'

	export let value: any
	let jsonViewer: Drawer
	let jsonViewerContent: any | undefined

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	async function getResource(path: string) {
		jsonViewerContent = await ResourceService.getResourceValue({
			workspace: $workspaceStore!,
			path
		})
	}

	async function getVariable(path: string) {
		jsonViewerContent = await VariableService.getVariableValue({
			workspace: $workspaceStore!,
			path
		})
	}
</script>

<Drawer bind:this={jsonViewer} size="800px">
	<DrawerContent title="Argument Details" on:close={jsonViewer.closeDrawer}>
		{#snippet actions()}
			<Button
				on:click={() => copyToClipboard(JSON.stringify(jsonViewerContent, null, 4))}
				color="light"
				size="xs"
				startIcon={{ icon: ClipboardCopy }}
			>
				Copy
			</Button>
		{/snippet}
		{#if isString(jsonViewerContent)}
			<pre>{jsonViewerContent}</pre>
		{:else}
			<ObjectViewer pureViewer json={jsonViewerContent} />
		{/if}
	</DrawerContent>
</Drawer>

{#if value == undefined || value == null}
	<span class="text-tertiary">null</span>
{:else if value === '<function call>'}
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
{:else if isString(value) && value.startsWith('$var:')}
	<button
		class="text-xs text-blue-500"
		on:click={async () => {
			await getVariable(value.substring('$res:'.length))
			jsonViewer.toggleDrawer()
		}}>{value}</button
	>
{:else if typeof value !== 'object'}
	<span>
		{truncate(JSON.stringify(value), 80)}
		{#if JSON.stringify(value).length > 80}
			<button
				class="text-xs text-blue-500"
				on:click={() => {
					jsonViewerContent = value
					jsonViewer.toggleDrawer()
				}}>See expanded</button
			>
		{/if}
	</span>
{:else}
	<div class="relative">
		{#if JSON.stringify(value).length > 120}
			<button
				class="text-xs absolute top-0 right-4 text-tertiary"
				on:click={() => {
					jsonViewerContent = value
					jsonViewer.toggleDrawer()
				}}><Expand size={18} /></button
			>
		{/if}
		<div class="max-h-60 overflow-auto">
			<ObjectViewer collapsed={false} topBrackets={true} pureViewer={true} json={value} />
		</div>
	</div>
{/if}
