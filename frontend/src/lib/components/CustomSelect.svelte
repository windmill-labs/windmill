<script lang="ts">
	import Select from '$lib/components/apps/svelte-select/lib/Select.svelte'
	import { twMerge } from 'tailwind-merge'

	export let items: any
	export let value: string
	export let placeholder: string = ''

	export let selectClass = ''

	let focused: boolean
</script>

<div
	class={twMerge(
		'min-w-[50px] rounded-md shadow-sm',
		focused ? 'ring ring-surface-hover' : '',
		selectClass
	)}
>
	<Select
		{items}
		clearable={false}
		{placeholder}
		bind:justValue={value}
		bind:focused
		listAutoWidth={true}
	/>
</div>

<style>
	:global(.svelte-select) {
		--font-size: 0.75rem !important;
		--line-height: 1rem !important;
		--height: 26px !important;
		--padding: 0 0.5rem;
		--selected-item-padding: 0 0;
		--background: rgb(var(--color-surface)) !important;
		--border-focused: 1px solid rgb(var(--color-border-hover)) !important;
		--border-hover: 1px solid rgb(var(--color-border-hover)) !important;
		--border: 1px solid rgb(var(--color-border)) !important;
		--placeholder-color: rgb(var(--color-text-tertiary)) !important;
		--selected-item-color: rgb(var(--color-text-primary)) !important;
	}

	:global(.svelte-select-list) {
		--item-color: rgb(var(--color-text-primary)) !important;
		--item-hover-bg: rgb(var(--color-surface-hover)) !important;
		--item-is-active-bg: rgb(var(--color-surface-selected)) !important;
		--list-background: rgb(var(--color-surface)) !important;
	}
</style>
