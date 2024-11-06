<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '../Popover.svelte'
	import { createEventDispatcher } from 'svelte'

	export let label: string | undefined = undefined
	export let numUnaknowledgedCriticalAlerts: number
	export let isCollapsed: boolean
	export let disabled: boolean = false
	export let lightMode: boolean = false
	export let stopPropagationOnClick: boolean = false
	export let shortcut: string = ''

	let dispatch = createEventDispatcher()
</script>

{#if !disabled}
	<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
		<div class="py-1.5 px-1 border-t border-gray-700">
		<button
			on:click={(e) => {
				if (stopPropagationOnClick) e.preventDefault()
				dispatch('click')
			}}
			class={twMerge(
				'group flex items-center px-2 py-2 font-light rounded-md h-8 gap-3 w-full',
				lightMode
					? 'text-primary hover:bg-surface-hover '
					: '  hover:bg-[#2A3648] text-primary-inverse dark:text-primary',
				'transition-all',
				$$props.class
			)}
			title={label}
		>
			<span
				class="flex items-center justify-center h-6 w-6 rounded-lg bg-red-600 text-white font-bold text-xs"
			>
				{numUnaknowledgedCriticalAlerts > 9 ? '9+' : numUnaknowledgedCriticalAlerts}
			</span>

			{#if !isCollapsed && label}
				<span
					class={twMerge(
						'whitespace-pre truncate',
						lightMode ? 'text-primary' : 'text-primary-inverse  dark:text-primary',
						'transition-all',
						$$props.class
					)}
				>
					{label}
					<span class="pl-2 text-xs dark:text-secondary light:text-secondary-inverse font-semibold">
						{shortcut}
					</span>
				</span>
			{/if}
		</button>
	</div>
		<svelte:fragment slot="text">
			{#if label}
				{label}
			{/if}
		</svelte:fragment>
	</Popover>
{/if}
