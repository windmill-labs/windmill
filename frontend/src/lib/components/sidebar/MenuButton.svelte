<script module lang="ts">
	export const sidebarClasses = {
		text: 'text-secondary text-xs font-normal',
		iconText: 'text-hint',
		selectedText: 'text-emphasis text-xs font-semibold',
		sublabelText: 'text-secondary text-2xs font-normal',
		hoverBg: 'transition-colors hover:bg-surface-hover',
		selectedBg: 'bg-surface-hover'
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
		sublabel?: string | undefined
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
		sublabel = undefined,
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
				'group flex items-center px-2 py-2 font-light rounded-md gap-2 w-full',
				sidebarClasses.hoverBg,
				'transition-all relative',
				sublabel ? 'h-10' : 'h-8',
				classNames
			)}
			use:conditionalMelt={trigger}
			aria-label={label}
			title={isCollapsed ? undefined : label}
			{...$trigger}
		>
			{#if icon}
				{@const SvelteComponent = icon}
				{#if color}
					<svg width="26" height="26" viewBox="0 0 26 26" class="flex-shrink-0 -ml-[5px]">
						<circle cx="13" cy="13" r="13" fill={color} />
						<foreignObject x="5" y="5" width="16" height="16">
							<SvelteComponent
								size={16}
								class={twMerge(sidebarClasses.iconText, 'transition-colors', iconClasses)}
								{...iconProps}
							/>
						</foreignObject>
					</svg>
				{:else}
					<SvelteComponent
						size={16}
						class={twMerge('flex-shrink-0', sidebarClasses.iconText, 'transition-colors', iconClasses)}
						{...iconProps}
					/>
				{/if}
			{/if}

			<div class="flex flex-col text-left grow min-w-0">
				{#if !isCollapsed && label}
					<div
						class={twMerge(
							'whitespace-pre truncate w-full',
							sidebarClasses.text,
							'transition-all',
							classNames
						)}
						title={label}
					>
						{label}
						<span
							class="pl-2 text-xs text-secondary font-semibold"
						>
							{shortcut}
						</span>
					</div>
				{/if}

				{#if sublabel}
					<div
						class={twMerge(
							'whitespace-pre truncate w-full',
							sidebarClasses.sublabelText,
							'transition-all',
							classNames
						)}
						title={sublabel}>{sublabel}</div
					>
				{/if}
			</div>

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
