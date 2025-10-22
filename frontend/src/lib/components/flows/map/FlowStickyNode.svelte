<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { DollarSign, Settings } from 'lucide-svelte'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import FlowAIButton from '$lib/components/copilot/chat/flow/FlowAIButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

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

<div class="flex flex-row gap-2 h-8 p-1 rounded-md bg-surface">
	{#if !disableSettings}
		<Button
			unifiedSize="sm"
			wrapperClasses="min-w-36"
			startIcon={{ icon: Settings }}
			selected={$selectedId?.startsWith('settings')}
			variant="default"
			title="Settings"
			onClick={() => ($selectedId = 'settings')}
		>
			Settings
			{#if flowStore.val.value.same_worker}
				<Badge color="blue" wrapperClass="max-h-[18px]">./shared</Badge>
			{/if}
		</Button>
	{/if}
	<Popover>
		<FlowErrorHandlerItem {disableAi} small={smallErrorHandler} on:generateStep />
		{#snippet text()}
			Error Handler
		{/snippet}
	</Popover>
	{#if !disableStaticInputs}
		<Popover>
			<Button
				wrapperClasses="h-full"
				unifiedSize="sm"
				startIcon={{ icon: DollarSign }}
				selected={$selectedId === 'constants'}
				variant="default"
				title="Static Inputs"
				iconOnly
				onClick={() => ($selectedId = 'constants')}
			/>
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
				selected={aiChatOpen}
			/>
			{#snippet text()}
				Flow AI Chat
			{/snippet}
		</Popover>
	{/if}
</div>
