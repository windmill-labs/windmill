<script lang="ts">
	import { truncate } from '$lib/utils'
	import Modal from './Modal.svelte'
	import Tooltip from './Tooltip.svelte'
	import json from 'svelte-highlight/languages/json'
	import github from 'svelte-highlight/styles/github'
	import { Highlight } from 'svelte-highlight'
	import { ResourceService, type Resource } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let value: any
	let resourceViewer: Modal
	let resource: Resource

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	async function getResource(path) {
		resource = await ResourceService.getResource({ workspace: $workspaceStore!, path })
	}

	let asJson: string = JSON.stringify(value, null, 4)
</script>

<svelte:head>
	{@html github}
</svelte:head>

<Modal bind:this={resourceViewer}>
	<div slot="title">{resource.path}</div>
	<div slot="content">
		<Highlight language={json} code={JSON.stringify(resource.value, null, 4)} />
	</div>
</Modal>

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
			resourceViewer.openModal()
		}}>{value}</button
	>{:else if asJson.length > 40}
	{truncate(asJson, 40)}<Tooltip>{asJson}</Tooltip>
{:else}
	{asJson}
{/if}
