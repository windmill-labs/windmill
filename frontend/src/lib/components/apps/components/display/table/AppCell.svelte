<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	type LinkObject = {
		href: string
		label: string
	}

	export let type: 'text' | 'badge' | 'link' = 'text'
	export let value: any
	export let width: number

	let isEditable = writable(false)
	let tempValue = value

	const dispatch = createEventDispatcher()

	function isLinkObject(value: any): value is LinkObject {
		return value && typeof value === 'object' && 'href' in value && 'label' in value
	}

	async function toggleEdit() {
		$isEditable = !$isEditable
		if ($isEditable) {
			await tick()

			const input = document.getElementById('cell') as HTMLInputElement

			input?.focus()
			input?.setSelectionRange(0, 9999)
		}
	}

	function handleInput(event: Event) {
		tempValue = (event.target as HTMLInputElement).value
	}

	function saveEdit() {
		value = tempValue

		dispatch('update', {
			value
		})

		toggleEdit()
	}
</script>

<td
	on:keydown
	on:click
	class={twMerge(
		'p-4 whitespace-pre-wrap truncate text-xs text-primary',
		$isEditable && 'bg-gray-100'
	)}
	style={'width: ' + width + 'px'}
>
	{#if type === 'badge'}
		<Badge>
			{value}
		</Badge>
	{:else if type === 'link'}
		{#if isLinkObject(value)}
			<a href={value.href} class="underline" target="_blank">
				{value.label}
			</a>
		{:else}
			<a href={value} class="underline" target="_blank">{value}</a>
		{/if}
	{:else if $isEditable}
		<input
			type="text"
			value={tempValue}
			on:input={handleInput}
			on:blur={saveEdit}
			id="cell"
			class="!appearance-none !bg-transparent !border-none !p-0 !m-0 leading-normal !text-xs"
			style="outline: none; box-shadow: none; height: auto; resize: none;"
			on:keypress={(e) => {
				if (e.key === 'Enter') {
					saveEdit()
				}
			}}
		/>
	{:else}
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div on:dblclick={toggleEdit}>
			{value}
		</div>
	{/if}
</td>
