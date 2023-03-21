<script lang="ts">
	import type { SchemaProperty } from '$lib/common'
	import { createEventDispatcher } from 'svelte'
	import { faPen, faTrash } from '@fortawesome/free-solid-svg-icons'
	import SchemaEditorProperty from './SchemaEditorProperty.svelte'
	import { Button } from './common'

	export let name: string
	export let property: SchemaProperty
	export let i: number
	export let propertiesNumber: number
	export let required: boolean
	export let isAnimated: boolean

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
</script>

<td class="font-bold">{name}</td>
<td>
	<SchemaEditorProperty {property} />
</td>
<td>{property.description ?? ''}</td>
<td>{property.default ? JSON.stringify(property.default) : ''}</td>
<td
	>{#if required}
		<span class="text-red-600 font-bold text-lg">*</span>
	{/if}</td
>
<td class="justify-end flex">
	{#if i > 0}
		<button
			on:click={() => changePosition(i, true)}
			class="text-lg mr-2 {isAnimated ? 'invisible' : ''}"
		>
			&uparrow;</button
		>
	{/if}
	{#if i < propertiesNumber - 1}
		<button
			on:click={() => changePosition(i, false)}
			class="text-lg mr-2 {isAnimated ? 'invisible' : ''}">&downarrow;</button
		>
	{/if}

	<Button
		color="red"
		variant="border"
		btnClasses="mx-2"
		size="sm"
		startIcon={{ icon: faTrash }}
		on:click={() => handleDeleteArgument(name)}
	>
		Delete
	</Button>
	<Button
		color="light"
		variant="border"
		size="sm"
		startIcon={{ icon: faPen }}
		on:click={() => startEditArgument(name)}
	>
		Edit
	</Button>
</td>
