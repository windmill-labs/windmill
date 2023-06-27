<script lang="ts">
	import { Pen } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let customValue: boolean
	export let disabled: boolean
	export let value: any
	export let enum_: string[] | undefined
	export let autofocus: boolean
	export let defaultValue: string | undefined
	export let valid: boolean

	const dispatch = createEventDispatcher()
</script>

{#if !customValue}
	<select
		on:focus={(e) => {
			dispatch('focus')
		}}
		{disabled}
		class="px-6"
		bind:value
	>
		{#each enum_ ?? [] as e}
			<option>{e}</option>
		{/each}
	</select>
{:else}
	<!-- svelte-ignore a11y-autofocus -->
	<input
		{autofocus}
		on:focus
		type="text"
		class={twMerge(
			'secondaryBackground',
			valid
				? ''
				: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'
		)}
		placeholder={defaultValue ?? ''}
		bind:value
	/>
{/if}

{#if !disabled}
	<button
		class="min-w-min !px-2 items-center text-gray-800 bg-gray-100 border rounded center-center hover:bg-gray-300 transition-all cursor-pointer"
		on:click={() => {
			customValue = !customValue
		}}
		title="Custom Value"
	>
		<Pen size={14} />
	</button>
{/if}
