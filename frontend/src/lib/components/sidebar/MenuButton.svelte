<script lang="ts">
	import { classNames } from '$lib/utils'
	import Popover from '../Popover.svelte'

	export let label: string
	export let icon: any | undefined = undefined
	export let isCollapsed: boolean
	export let disabled: boolean = false

	let isSelected = false
</script>

{#if !disabled}
	<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
		<button
			class={classNames(
				'group flex items-center px-2 py-2 font-light rounded-md h-8 gap-3 w-full',
				isSelected ? 'bg-[#30404e] hover:bg-[#30404e]' : 'hover:bg-[#34363c]',
				'transition-all',
				$$props.class
			)}
			title={label}
		>
			{#if icon}
				<svelte:component
					this={icon}
					size={16}
					class={classNames(
						'flex-shrink-0',
						isSelected
							? 'text-blue-100 group-hover:text-white'
							: 'text-gray-100 group-hover:text-white',
						'transition-all'
					)}
				/>
			{/if}

			{#if !isCollapsed}
				<span
					class={classNames(
						'whitespace-pre truncate',
						isSelected
							? 'text-blue-100 group-hover:text-white font-semibold'
							: 'text-gray-100 group-hover:text-white',
						'transition-all',
						$$props.class
					)}
				>
					{label}
				</span>
			{/if}
		</button>
		<svelte:fragment slot="text">
			{label}
		</svelte:fragment>
	</Popover>
{/if}
