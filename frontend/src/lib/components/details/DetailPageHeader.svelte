<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'
	import { twMerge } from 'tailwind-merge'
	import { userStore } from '$lib/stores'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import type { TriggerContext } from '../triggers'
	import { Calendar } from 'lucide-svelte'

	type MainButton = {
		label: string
		href: string
		buttonProps: ButtonProps
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
		onSaved,
		children,
		trigger_badges
	}: Props = $props()

	const dispatch = createEventDispatcher()
</script>

<div class="border-b">
	<div class="mx-auto">
		<div
			class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center min-h-12"
		>
			<div class="grow px-2 inline-flex items-center gap-4 min-w-0">
				<div class={twMerge('min-w-0', $userStore?.operator ? 'pl-10' : '')}>
					<SummaryPathDisplay {summary} {path} {onSaved} kind={errorHandlerKind} />
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
				{#if menuItems.length > 0}
					{#key menuItems}
						<DropdownV2
							items={menuItems.map((item) => ({
								displayName: item.label,
								icon: item.Icon,
								action: item.onclick,
								type: item.color === 'red' ? 'delete' : 'action'
							}))}
							placement="bottom-end"
							size="md"
						/>
					{/key}
				{/if}
				<ErrorHandlerToggleButton
					kind={errorHandlerKind}
					{scriptOrFlowPath}
					bind:errorHandlerMuted
				/>
				{#each mainButtons as btn}
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						btnClasses="hidden md:flex items-center gap-1 whitespace-nowrap"
					>
						{btn.label}
					</Button>
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						iconOnly
						btnClasses="flex md:hidden items-center gap-1 whitespace-nowrap"
					>
						{btn.label}
					</Button>
				{/each}
			</div>
		</div>
	</div>
</div>
