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

	function handleDeleteArgument(argPath: string[]) {
		dispatch('deleteArgument', argPath)
	}

	function changePosition(i: number, up: boolean) {
		dispatch('changePosition', { i, up })
	}

	function getArgPath(displayInfo: PropertyDisplayInfo): string[] {
		return [...displayInfo.path, displayInfo.name]
	}

	const indentStyle = depth > 0 ? `margin-left :${depth}rem` : ''
</script>

<td class="font-bold">
	<span style={indentStyle}>
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
			color="light"
			variant="border"
			size="sm"
			startIcon={{ icon: faPen }}
			on:click={() => startEditArgument(displayInfo.name)}
		>
			Edit
		</Button>
	{/if}
	<Button
		color="red"
		variant="border"
		btnClasses="mx-2"
		size="sm"
		startIcon={{ icon: faTrash }}
		on:click={() => handleDeleteArgument(getArgPath(displayInfo))}
	>
		Delete
	</Button>
</td>
