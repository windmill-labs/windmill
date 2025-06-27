<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { Badge } from '$lib/components/common'
	import { DollarSign, Settings } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import FlowAIButton from '$lib/components/copilot/chat/flow/FlowAIButton.svelte'
	import Popover from '$lib/components/Popover.svelte'

	interface Props {
		disableSettings?: boolean
		disableStaticInputs?: boolean
		smallErrorHandler: boolean
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
	}

	let {
		disableSettings,
		disableStaticInputs,
		smallErrorHandler,
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat
	}: Props = $props()

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const nodeClass =
		'border w-fit rounded p-1 px-2 bg-surface text-sm cursor-pointer flex items-center h-[28px] hover:!bg-surface-secondary active:!bg-surface'
	const nodeSelectedClass =
		'outline outline-offset-1 outline-2 outline-slate-800 dark:bg-white/5 dark:outline-slate-400/60 dark:outline-gray-400'
</script>

<div class="flex flex-row gap-2 p-1 rounded shadow-md bg-surface">
	{#if !disableSettings}
		<button
			onclick={() => ($selectedId = 'settings-metadata')}
			class={twMerge(nodeClass, $selectedId?.startsWith('settings') ? nodeSelectedClass : '')}
			title="Settings"
		>
			<Settings size={14} />
			<span
				class="font-bold flex flex-row justify-between w-fit gap-2 items-center truncate ml-1.5"
			>
				<span class="text-xs">Settings</span>
				<span class="h-[18px] flex items-center">
					{#if flowStore.val.value.same_worker}
						<Badge color="blue" baseClass="truncate">./shared</Badge>
					{/if}
				</span>
			</span>
		</button>
	{/if}
	{#if !disableStaticInputs}
		<Popover>
			<button
				onclick={() => ($selectedId = 'constants')}
				class={twMerge(nodeClass, $selectedId == 'constants' ? nodeSelectedClass : '')}
			>
				<DollarSign size={14} />
			</button>
			{#snippet text()}
				Static Inputs
			{/snippet}
		</Popover>
	{/if}
	<Popover>
		<FlowErrorHandlerItem small={smallErrorHandler} on:generateStep clazz={nodeClass} />
		{#snippet text()}
			Error Handler
		{/snippet}
	</Popover>
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
