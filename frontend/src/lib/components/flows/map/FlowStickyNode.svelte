<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import type { FlowDiffManager } from '../flowDiffManager.svelte'
	import { getContext } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { DollarSign, Settings, StickyNote } from 'lucide-svelte'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import AIButton from '$lib/components/copilot/chat/AIButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'

	interface Props {
		disableSettings?: boolean
		disableStaticInputs?: boolean
		smallErrorHandler: boolean
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
		noteMode?: boolean
		toggleNoteMode?: () => void
		disableAi?: boolean
		diffManager?: FlowDiffManager
	}

	let {
		disableSettings,
		disableStaticInputs,
		smallErrorHandler,
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat,
		noteMode,
		toggleNoteMode,
		disableAi,
		diffManager
	}: Props = $props()

	const { selectionManager, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const selectedId = $derived(selectionManager.getSelectedId())
</script>

<div class="flex flex-row gap-2 p-1 rounded-md bg-surface">
	{#if !disableSettings}
		<Button
			unifiedSize="sm"
			wrapperClasses="min-w-36"
			startIcon={{ icon: Settings }}
			selected={selectedId?.startsWith('settings')}
			variant="default"
			title="Settings"
			onClick={() => selectionManager.selectId('settings')}
		>
			Settings
			{#if flowStore.val.value.same_worker}
				<Badge color="blue" wrapperClass="max-h-[18px]">./shared</Badge>
			{/if}
		</Button>
	{/if}
	<Popover>
		<FlowErrorHandlerItem {disableAi} small={smallErrorHandler} {diffManager} on:generateStep />
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
				selected={selectedId === 'constants'}
				variant="default"
				iconOnly
				onClick={() => selectionManager.selectId('constants')}
			/>
			{#snippet text()}
				Environment Variables
			{/snippet}
		</Popover>
	{/if}
	{#if showFlowAiButton}
		<Popover>
			<AIButton
				togglePanel={() => {
					toggleAiChat?.()
				}}
				btnClasses={AIBtnClasses(aiChatOpen ? 'selected' : 'default')}
			/>
			{#snippet text()}
				Flow AI Chat
			{/snippet}
		</Popover>
	{/if}
	<Popover>
		<Button
			onclick={() => toggleNoteMode?.()}
			iconOnly
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: StickyNote }}
			selected={noteMode}
		></Button>
		{#snippet text()}
			{noteMode ? 'Exit note mode' : 'Add sticky notes'}
		{/snippet}
	</Popover>
</div>
