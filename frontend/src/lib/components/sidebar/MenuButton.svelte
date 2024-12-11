<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '../Popover.svelte'
	import { createEventDispatcher } from 'svelte'
	import SideBarNotification from './SideBarNotification.svelte'
	export let label: string | undefined = undefined
	export let icon: any | undefined = undefined
	export let isCollapsed: boolean
	export let disabled: boolean = false
	export let lightMode: boolean = false
	export let stopPropagationOnClick: boolean = false
	export let shortcut: string = ''
	export let notificationsCount: number = 0

	let dispatch = createEventDispatcher()
</script>

{#if !disabled}
	<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
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
			{#if icon}
				<svelte:component
					this={icon}
					size={16}
					class={twMerge(
						'flex-shrink-0',
						lightMode
							? 'text-primary group-hover:text-secondary'
							: 'text-primary-inverse group-hover:text-secondary-inverse dark:group-hover:text-secondary dark:text-primary',
						'transition-all'
					)}
				/>
			{/if}

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

			{#if isCollapsed && notificationsCount > 0}
				<div class="absolute translate-x-1/2 translate-y-1/2 -top-2 right-1 flex h-fit w-fit">
					<SideBarNotification notificationCount={notificationsCount} small={true} />
				</div>
			{:else if notificationsCount > 0}
				<div class="ml-auto">
					<SideBarNotification notificationCount={notificationsCount} small={false} />
				</div>
			{/if}
		</button>
		<svelte:fragment slot="text">
			{#if label}
				{label}
			{/if}
		</svelte:fragment>
	</Popover>
{/if}
