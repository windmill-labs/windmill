<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'
	import { twMerge } from 'tailwind-merge'
	import { userStore } from '$lib/stores'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { TriggerContext } from '../triggers'
	import { Calendar, Pen } from 'lucide-svelte'
	import { emptyString } from '$lib/utils'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

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
		onEdit?: (summary: string, path: string) => void
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
		onEdit,
		children,
		trigger_badges
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let editSummary = $state('')
	let editPath = $state('')
	let popoverOpen = $state(false)

	$effect(() => {
		if (popoverOpen) {
			editSummary = summary ?? ''
			editPath = path ?? ''
		}
	})
</script>

<div class="border-b p-1">
	<div class="mx-auto">
		<div
			class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center min-h-10"
		>
			<div class="grow px-4 inline-flex items-center gap-4 min-w-0">
				<div class="inline-flex items-center gap-1 min-w-0">
					<div
						class={twMerge(
							'min-w-24 text-emphasis truncate flex flex-col gap-0',
							$userStore?.operator ? 'pl-10' : ''
						)}
					>
						<span
							class={twMerge(
								'text-sm min-w-24 text-emphasis font-semibold truncate',
								$userStore?.operator ? 'pl-10' : ''
							)}
						>
							{emptyString(summary) ? (path ?? '') : summary}
						</span>
						{#if !emptyString(summary)}
							<span class="text-2xs text-secondary">{path}</span>
						{/if}
					</div>
					{#if onEdit}
						<Popover
							placement="bottom-start"
							contentClasses="p-4"
							usePointerDownOutside
							bind:isOpen={popoverOpen}
						>
							{#snippet trigger()}
								<span
									class="p-1 rounded hover:bg-surface-hover text-secondary hover:text-primary flex-shrink-0"
									title="Edit summary and path"
								>
									<Pen size={14} />
								</span>
							{/snippet}
							{#snippet content({ close })}
								<div class="flex flex-col gap-3 w-72">
									<label class="block text-primary">
										<div class="pb-1 text-xs font-semibold text-emphasis">Summary</div>
										<TextInput
											inputProps={{
												type: 'text',
												placeholder: 'Short summary',
												onkeydown: (e) => {
													if (e.key === 'Enter') {
														onEdit?.(editSummary, editPath)
														close()
													}
												}
											}}
											bind:value={editSummary}
										/>
									</label>
									<label class="block text-primary">
										<div class="pb-1 text-xs font-semibold text-emphasis">Path</div>
										<TextInput
											inputProps={{
												type: 'text',
												placeholder: 'Path',
												onkeydown: (e) => {
													if (e.key === 'Enter') {
														onEdit?.(editSummary, editPath)
														close()
													}
												}
											}}
											bind:value={editPath}
										/>
									</label>
									<Button
										size="xs"
										on:click={() => {
											onEdit?.(editSummary, editPath)
											close()
										}}
									>
										Save
									</Button>
								</div>
							{/snippet}
						</Popover>
					{/if}
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
			<div class="flex gap-1 items-center">
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
