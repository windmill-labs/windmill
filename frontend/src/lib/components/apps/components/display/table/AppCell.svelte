<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import { isLinkObject } from './utils'

	interface Props {
		type?: 'text' | 'badge' | 'link'
		value: any
		width: number
	}

	let { type = 'text', value = $bindable(), width }: Props = $props()

	let isEditable = writable(false)
	let tempValue = $state(value)

	const dispatch = createEventDispatcher()

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
	onkeydown={bubble('keydown')}
	onclick={bubble('click')}
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
			oninput={handleInput}
			onblur={saveEdit}
			id="cell"
			class="!appearance-none !bg-transparent !border-none !p-0 !m-0 leading-normal !text-xs"
			style="outline: none; box-shadow: none; height: auto; resize: none;"
			onkeypress={(e) => {
				if (e.key === 'Enter') {
					saveEdit()
				}
			}}
		/>
	{:else}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div ondblclick={toggleEdit}>
			{#if typeof value == 'object'}
				{JSON.stringify(value)}
			{:else}
				{value}
			{/if}
		</div>
	{/if}
</td>
