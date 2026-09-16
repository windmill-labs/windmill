<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'
	import { twMerge } from 'tailwind-merge'
	import { userStore } from '$lib/stores'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import { MediaQuery } from 'svelte/reactivity'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import type { TriggerContext } from '../triggers'
	import type { Item } from '$lib/utils'
	import { Bell, BellOff, Calendar } from 'lucide-svelte'
	import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'

	type MainButton = {
		label: string
		/** Shown under the label once the button has collapsed into a menu. */
		description?: string
		buttonProps: ButtonProps
		/** Where the button lands below the `lg` breakpoint, where the bar cannot hold every
		 * button beside the summary: the ellipsis menu, or the dropdown of the enabled bar button
		 * labelled `dropdownOf` (the menu when there is none). Unset keeps it in the bar at every
		 * width. */
		narrow?: 'menu' | { dropdownOf: string }
	}

	type ButtonProps = any
	type MenuItemButton = {
		label: string
		Icon: any
		onclick: () => void
		color?: 'red'
	}

	const { triggersCount, triggersState } = $state(getContext<TriggerContext>('TriggerContext'))

	interface Props {
		mainButtons?: MainButton[]
		menuItems?: MenuItemButton[]
		summary?: string
		path?: string
		tag: string | undefined
		errorHandlerKind: 'flow' | 'script'
		scriptOrFlowPath: string
		errorHandlerMuted: boolean | undefined
		labels?: string[] | undefined
		inheritedLabels?: string[] | undefined
		onSaved?: (newPath: string) => void
		children?: import('svelte').Snippet
		trigger_badges?: import('svelte').Snippet
	}

	let {
		mainButtons = [],
		menuItems = [],
		summary,
		path,
		tag,
		errorHandlerKind,
		scriptOrFlowPath,
		errorHandlerMuted = $bindable(),
		labels = $bindable(),
		inheritedLabels = undefined,
		onSaved,
		children,
		trigger_badges
	}: Props = $props()

	const dispatch = createEventDispatcher()

	// Tailwind's `lg`, matched in JS so the one ellipsis menu can carry the collapsed buttons.
	const wide = new MediaQuery('(min-width: 1024px)')

	const barButtons = $derived(wide.current ? mainButtons : mainButtons.filter((b) => !b.narrow))

	function dropdownHost(btn: MainButton): MainButton | undefined {
		if (typeof btn.narrow !== 'object') return undefined
		const label = btn.narrow.dropdownOf
		return barButtons.find((b) => b.label === label && !b.buttonProps.disabled)
	}

	async function toggleErrorHandler() {
		const next = await toggleWorkspaceErrorHandler(
			errorHandlerKind,
			scriptOrFlowPath,
			errorHandlerMuted
		)
		if (next !== undefined) errorHandlerMuted = next
	}

	const allMenuItems: Item[] = $derived([
		...(wide.current ? [] : mainButtons.filter((b) => b.narrow && !dropdownHost(b))).map((b) => ({
			displayName: b.label,
			description: b.description,
			icon: b.buttonProps.startIcon,
			href: b.buttonProps.href,
			action: b.buttonProps.onClick,
			disabled: b.buttonProps.disabled,
			type: 'action' as const
		})),
		...(wide.current
			? []
			: [
					{
						displayName: errorHandlerMuted ? 'Unmute error handler' : 'Mute error handler',
						icon: errorHandlerMuted ? BellOff : Bell,
						action: toggleErrorHandler,
						type: 'action' as const
					}
				]),
		...menuItems.map((item, i) => ({
			displayName: item.label,
			icon: item.Icon,
			action: item.onclick,
			type: item.color === 'red' ? ('delete' as const) : ('action' as const),
			separatorTop: i === 0 && !wide.current
		}))
	])

	function dropdownItemsOf(host: MainButton) {
		if (wide.current) return undefined
		const items = mainButtons
			.filter((b) => dropdownHost(b) === host)
			.map((b) => ({
				label: b.label,
				description: b.description,
				icon: b.buttonProps.startIcon,
				href: b.buttonProps.href,
				onClick: b.buttonProps.onClick,
				disabled: b.buttonProps.disabled
			}))
		return items.length > 0 ? items : undefined
	}
</script>

<div class="border-b">
	<div class="mx-auto">
		<div
			class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center min-h-12"
		>
			<div class="grow px-2 inline-flex items-center gap-4 min-w-0">
				<div class={twMerge('min-w-0', $userStore?.operator ? 'pl-10' : '')}>
					<SummaryPathDisplay
						{summary}
						{path}
						bind:labels
						{inheritedLabels}
						{onSaved}
						kind={errorHandlerKind}
					/>
				</div>
				{#if tag}
					<Badge>tag: {tag}</Badge>
				{/if}
				{@render children?.()}
				{#if triggersState?.triggers?.some((t) => t.isPrimary && !t.isDraft)}
					{@const primarySchedule = triggersState.triggers.findIndex(
						(t) => t.isPrimary && !t.isDraft
					)}
					<Button
						btnClasses="inline-flex"
						startIcon={{ icon: Calendar }}
						variant="contained"
						color="light"
						size="xs"
						on:click={async () => {
							dispatch('seeTriggers')
							await tick()
							triggersState.selectedTriggerIndex = primarySchedule
						}}
					>
						{$triggersCount?.primary_schedule?.schedule ?? ''}
					</Button>
				{/if}
				{@render trigger_badges?.()}
			</div>
			<div class="flex gap-1 items-center pr-4">
				{#if allMenuItems.length > 0}
					{#key allMenuItems}
						<DropdownV2 items={allMenuItems} placement="bottom-end" size="md" />
					{/key}
				{/if}
				{#if wide.current}
					<ErrorHandlerToggleButton
						kind={errorHandlerKind}
						{scriptOrFlowPath}
						bind:errorHandlerMuted
					/>
				{/if}
				{#each barButtons as btn (btn.label)}
					{@const dropdownItems = dropdownItemsOf(btn)}
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						{dropdownItems}
						dropdownWidth={dropdownItems?.some((i) => i.description) ? 288 : undefined}
						btnClasses="flex items-center gap-1 whitespace-nowrap"
					>
						{btn.label}
					</Button>
				{/each}
			</div>
		</div>
	</div>
</div>
