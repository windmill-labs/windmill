<script lang="ts">
	import { getContext } from 'svelte'
	import { Tab } from '@rgossiaux/svelte-headlessui'
	import type { ToggleButtonContext } from './ToggleButtonGroup.svelte'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'

	export let value: any
	export let label: string | undefined = undefined
	export let iconOnly: boolean = false
	export let tooltip: string | undefined = undefined
	export let icon: any | undefined = undefined
	export let disabled: boolean = false
	export let selectedColor: string = '#3b82f6'
	export let small: boolean = false
	export let iconProps: Record<string, any> = {}

	const { select, selected } = getContext<ToggleButtonContext>('ToggleButtonGroup')
</script>

<Popover
	notClickable
	class={twMerge('flex', disabled ? 'cursor-not-allowed' : 'cursor-pointer')}
	disablePopup={tooltip === undefined}
	disappearTimeout={0}
>
	<Tab
		{disabled}
		class={twMerge(
			' rounded-md transition-all text-xs flex gap-1 flex-row items-center',
			small ? 'px-1 py-0.5' : 'px-2 py-1',
			$selected === value ? 'bg-surface shadow-md' : 'bg-surface-secondary hover:bg-surface-hover',
			$$props.class
		)}
		on:click={() => select(value)}
	>
		{#if icon}
			<svelte:component
				this={icon}
				size={14}
				color={$selected === value ? selectedColor : '#9CA3AF'}
				{...iconProps}
			/>
		{/if}
		{#if label && !iconOnly}
			{label}
		{/if}
	</Tab>

	<svelte:fragment slot="text">
		{tooltip ?? label}
	</svelte:fragment>
</Popover>
