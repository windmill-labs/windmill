<script module lang="ts">
	export const sidebarClasses = {
		text: 'text-primary-inverse dark:text-primary data-[light-mode=true]:text-primary text-xs font-normal',
		selectedText: 'text-emphasis-inverse dark:text-emphasis text-xs font-normal',
		hoverBg:
			'transition-colors hover:bg-surface-hover-inverse dark:hover:bg-surface-hover data-[light-mode=true]:hover:bg-surface-hover'
	}
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '../Popover.svelte'
	import { createEventDispatcher } from 'svelte'
	import SideBarNotification from './SideBarNotification.svelte'
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

	let buttonRef: HTMLButtonElement | HTMLAnchorElement | undefined = $state(undefined)

	let dispatch = createEventDispatcher()

	// Dynamic component based on whether href is provided
	const Element = href ? 'a' : 'button'
</script>

{#if !disabled}
	<Popover
		appearTimeout={0}
		disappearTimeout={0}
		class="w-full"
		disablePopup={!isCollapsed}
		placement="right"
	>
		<svelte:element
			this={Element}
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
			onclick={href
				? undefined
				: (e) => {
						if (stopPropagationOnClick) e.preventDefault()
						dispatch('click')
					}}
			{href}
			data-light-mode={lightMode}
			class={twMerge(
				'group flex items-center px-2 py-2 font-light rounded-md h-8 gap-3 w-full',
				sidebarClasses.hoverBg,
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
					class={twMerge('flex-shrink-0', sidebarClasses.text, 'transition-colors', iconClasses)}
					{...iconProps}
				/>
			{/if}

			{#if !isCollapsed && label}
				<span
					class={twMerge(
						'whitespace-pre truncate',
						sidebarClasses.text,
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
		</svelte:element>

		{#snippet text()}
			{#if label}
				{label}
			{/if}
		{/snippet}
	</Popover>
{/if}
