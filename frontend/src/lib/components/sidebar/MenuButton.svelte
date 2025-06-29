<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '../Popover.svelte'
	import { createEventDispatcher } from 'svelte'
	import SideBarNotification from './SideBarNotification.svelte'
	import { goto } from '$app/navigation'
	import { conditionalMelt } from '$lib/utils'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		label?: string | undefined
		icon?: any | undefined
		iconClasses?: string | null
		iconProps?: any | null
		isCollapsed: boolean
		disabled?: boolean
		lightMode?: boolean
		stopPropagationOnClick?: boolean
		shortcut?: string
		notificationsCount?: number
		color?: string | null
		trigger?: MenubarMenuElements['trigger'] | undefined
		href?: string | undefined
		class?: string | undefined
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		label = undefined,
		icon = undefined,
		iconClasses = null,
		iconProps = null,
		isCollapsed,
		disabled = false,
		lightMode = false,
		stopPropagationOnClick = false,
		shortcut = '',
		notificationsCount = 0,
		color = null,
		trigger = undefined,
		href = undefined,
		class: classNames = undefined
	}: Props = $props()

	let buttonRef: HTMLButtonElement | undefined = $state(undefined)

	let dispatch = createEventDispatcher()
</script>

{#if !disabled}
	<Popover
		appearTimeout={0}
		disappearTimeout={0}
		class="w-full"
		disablePopup={!isCollapsed}
		placement="right"
	>
		<button
			bind:this={buttonRef}
			use:triggerableByAI={{
				id: aiId,
				description: aiDescription,
				callback: () => {
					if (buttonRef) {
						buttonRef.click()
					}
				}
			}}
			onclick={(e) => {
				if (stopPropagationOnClick) e.preventDefault()
				if (href) {
					goto(href)
				}
				dispatch('click')
			}}
			class={twMerge(
				'group flex items-center px-2 py-2 font-light rounded-md h-8 gap-3 w-full',
				lightMode
					? 'text-primary data-[highlighted]:bg-surface-hover hover:bg-surface-hover'
					: 'data-[highlighted]:bg-[#2A3648] hover:bg-[#2A3648] text-primary-inverse dark:text-primary',
				color ? 'border-4' : '',
				'transition-all relative',
				classNames
			)}
			style={color ? `border-color: ${color}; padding: 0 calc(0.5rem - 4px);` : ''}
			use:conditionalMelt={trigger}
			title={isCollapsed ? undefined : label}
			{...$trigger}
		>
			{#if icon}
				{@const SvelteComponent = icon}
				<SvelteComponent
					size={16}
					class={twMerge(
						'flex-shrink-0',
						lightMode
							? 'text-primary group-hover:text-secondary'
							: 'text-primary-inverse group-hover:text-secondary-inverse dark:group-hover:text-secondary dark:text-primary',
						'transition-all',
						iconClasses
					)}
					{...iconProps}
				/>
			{/if}

			{#if !isCollapsed && label}
				<span
					class={twMerge(
						'whitespace-pre truncate',
						lightMode ? 'text-primary' : 'text-primary-inverse dark:text-primary',
						'transition-all',
						classNames
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

		{#snippet text()}
			{#if label}
				{label}
			{/if}
		{/snippet}
	</Popover>
{/if}
