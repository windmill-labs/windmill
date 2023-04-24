<script lang="ts">
	import Required from './Required.svelte'
	import type { PropertyDisplayInfo } from '$lib/common'
	import { createEventDispatcher } from 'svelte'
	import { faPen, faTrash } from '@fortawesome/free-solid-svg-icons'
	import SchemaEditorProperty from './SchemaEditorProperty.svelte'
	import { Button } from './common'

	export let displayInfo: PropertyDisplayInfo
	export let isAnimated: boolean

	let depth = displayInfo.path.length
	const required = displayInfo.isRequired

	const dispatch = createEventDispatcher()

	function startEditArgument(name: string) {
		dispatch('startEditArgument', name)
	}

	function handleDeleteArgument(name: string) {
		dispatch('deleteArgument', name)
	}

	function changePosition(i: number, up: boolean) {
		dispatch('changePosition', { i, up })
	}

	const indentClass = depth > 0 ? `ml-${4 * depth}` : ''
</script>

<td class="font-bold">
	<span class={indentClass}>
		{displayInfo.name}
		<Required {required} class="!ml-0" />
	</span>
</td>
<td>
	<SchemaEditorProperty property={displayInfo.property} />
</td>
<td>{displayInfo.property.default ? JSON.stringify(displayInfo.property.default) : ''}</td>
<td>{displayInfo.property.description ?? ''}</td>
<td />
<td class="justify-end flex">
	{#if depth === 0}
		{#if displayInfo.index > 0}
			<button
				on:click={() => changePosition(displayInfo.index, true)}
				class="text-lg mr-2 {isAnimated ? 'invisible' : ''}"
			>
				&uparrow;</button
			>
		{/if}
		{#if displayInfo.index < displayInfo.propertiesNumber - 1}
			<button
				on:click={() => changePosition(displayInfo.index, false)}
				class="text-lg mr-2 {isAnimated ? 'invisible' : ''}">&downarrow;</button
			>
		{/if}

		<Button
			color="red"
			variant="border"
			btnClasses="mx-2"
			size="sm"
			startIcon={{ icon: faTrash }}
			on:click={() => handleDeleteArgument(displayInfo.name)}
		>
			Delete
		</Button>
		<Button
			color="light"
			variant="border"
			size="sm"
			startIcon={{ icon: faPen }}
			on:click={() => startEditArgument(displayInfo.name)}
		>
			Edit
		</Button>
	{/if}
</td>
