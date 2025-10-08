<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { DollarSign, Settings } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import FlowAIButton from '$lib/components/copilot/chat/flow/FlowAIButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { type IconType } from '$lib/utils'

	interface Props {
		disableSettings?: boolean
		disableStaticInputs?: boolean
		smallErrorHandler: boolean
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
		disableAi?: boolean
	}

	let {
		disableSettings,
		disableStaticInputs,
		smallErrorHandler,
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat,
		disableAi
	}: Props = $props()

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<div class="flex flex-row gap-1 p-1 rounded-md bg-surface">
	{#if !disableSettings}
		{@render btn({
			selected: $selectedId?.startsWith('settings'),
			onPress: () => ($selectedId = 'settings'),
			title: 'Settings',
			Icon: Settings
		})}
	{/if}
	<Popover>
		<FlowErrorHandlerItem
			{disableAi}
			small={smallErrorHandler}
			on:generateStep
			clazz={'border w-fit rounded p-1 px-2 bg-surface text-sm cursor-pointer flex items-center h-[28px] hover:!bg-surface-secondary active:!bg-surface'}
		/>
		{#snippet text()}
			Error Handler
		{/snippet}
	</Popover>
	{#if !disableStaticInputs}
		<Popover>
			{@render btn({
				selected: $selectedId == 'constants',
				onPress: () => ($selectedId = 'constants'),
				title: 'Static Inputs',
				Icon: DollarSign,
				iconOnly: true
			})}
			{#snippet text()}
				Static Inputs
			{/snippet}
		</Popover>
	{/if}
	{#if showFlowAiButton}
		<Popover>
			<FlowAIButton
				togglePanel={() => {
					toggleAiChat?.()
				}}
				opened={aiChatOpen}
			/>
			{#snippet text()}
				Flow AI Chat
			{/snippet}
		</Popover>
	{/if}
</div>

{#snippet btn({
	selected,
	onPress,
	title,
	iconOnly,
	Icon
}: {
	selected?: boolean
	onPress?: () => void
	title: string
	iconOnly?: boolean
	Icon: IconType
})}
	<button
		onclick={onPress}
		class={twMerge(
			'flex gap-2 items-center font-normal border rounded-md justify-center px-2 h-8',
			selected ? '' : '',
			iconOnly ? 'w-8' : 'min-w-36'
		)}
		{title}
	>
		<Icon size={14} />
		{#if !iconOnly}
			<span class="text-xs">{title}</span>
		{/if}
		{#if flowStore.val.value.same_worker}
			<Badge color="blue" wrapperClass="h-[18px]" baseClass="truncate">./shared</Badge>
		{/if}
	</button>
{/snippet}
