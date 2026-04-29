<script lang="ts">
	import {
		Loader2,
		ChevronDown,
		ChevronRight,
		XCircle,
		Play,
		ArrowUpRight,
		Workflow,
		Calendar,
		FileCode,
		LayoutDashboard
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { ToolDisplayMessage } from './shared'
	import { twMerge } from 'tailwind-merge'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { openPreviewModal } from './previewModalState.svelte'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const hasParameters = $derived(
		message.parameters !== undefined && Object.keys(message.parameters).length > 0
	)

	let isExpanded = $derived(
		message.showDetails ||
			(message.isStreamingArguments && hasParameters) ||
			(message.isLoading && message.needsConfirmation)
	)

	// Codex-style "open the resource" preview card under tool calls that
	// create or modify a flow step, schedule, script, or app. Inferred from
	// the tool name; the parameters give us the resource title when available.
	type PreviewKind = 'flow' | 'schedule' | 'script' | 'app'
	type PreviewInfo = {
		kind: PreviewKind
		title: string
		subtitle: string
		icon: typeof Workflow
		iconBg: string
		iconColor: string
		// When set, render LanguageIcon for that lang instead of the lucide icon.
		language?: string
	}

	const previewInfo = $derived.by((): PreviewInfo | undefined => {
		// Only show for create/edit/update tool calls that have completed successfully.
		const name = (message.toolName ?? '').toLowerCase()
		const verbMatch = /\b(create|add|edit|update|modify|set)/.test(name)
		if (!verbMatch) return undefined
		if (message.isLoading || message.error || message.needsConfirmation) return undefined

		const params = (message.parameters ?? {}) as Record<string, any>

		if (/(schedule|cron)/.test(name)) {
			return {
				kind: 'schedule',
				title: params.summary ?? params.path ?? 'Schedule',
				subtitle: 'Schedule',
				icon: Calendar,
				iconBg: 'bg-blue-50 dark:bg-blue-900/20',
				iconColor: 'text-blue-500'
			}
		}
		if (/(script)/.test(name)) {
			return {
				kind: 'script',
				title: params.summary ?? params.path ?? 'Script',
				subtitle: 'Script',
				icon: FileCode,
				iconBg: 'bg-violet-50 dark:bg-violet-900/20',
				iconColor: 'text-violet-500'
			}
		}
		if (/(app)/.test(name)) {
			return {
				kind: 'app',
				title: params.summary ?? params.path ?? 'App',
				subtitle: 'App',
				icon: LayoutDashboard,
				iconBg: 'bg-emerald-50 dark:bg-emerald-900/20',
				iconColor: 'text-emerald-500'
			}
		}
		if (/(flow|step|module)/.test(name)) {
			return {
				kind: 'flow',
				title: params.summary ?? params.id ?? params.path ?? 'Flow step',
				subtitle: /step|module/.test(name) ? 'Flow step' : 'Flow',
				icon: Workflow,
				iconBg: 'bg-sky-50 dark:bg-sky-900/20',
				iconColor: 'text-sky-500',
				language: typeof params.language === 'string' ? params.language : undefined
			}
		}
		return undefined
	})
</script>

<div class="flex flex-col gap-1.5">
	<div
		class="bg-surface border border-gray-200 dark:border-gray-700 rounded-md overflow-hidden font-mono text-xs"
	>
		<!-- Collapsible Header -->
		<button
			class={twMerge(
				'w-full p-2 bg-surface-secondary hover:bg-surface-hover transition-colors flex items-center justify-between text-left border-b border-gray-200 dark:border-gray-700',
				message.needsConfirmation ? 'opacity-80' : ''
			)}
			onclick={() => (isExpanded = !isExpanded)}
			disabled={!message.showDetails && !message.isStreamingArguments}
		>
			<div class="flex items-center gap-2 flex-1">
				{#if message.showDetails || message.isStreamingArguments}
					{#if isExpanded}
						<ChevronDown class="w-3 h-3 text-secondary" />
					{:else}
						<ChevronRight class="w-3 h-3 text-secondary" />
					{/if}
				{/if}

				{#if message.isLoading && !message.needsConfirmation}
					<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
				{:else if message.error}
					<span class="text-red-500">✗</span>
				{:else if !message.isLoading && !message.error}
					<span class="text-green-500">✓</span>
				{/if}
				<span class="text-primary font-medium text-xs">
					{message.content}
				</span>
			</div>
		</button>

		<!-- Expanded Content -->
		{#if isExpanded}
			<div class="p-2 bg-surface space-y-3">
				<!-- Parameters Section - show if we have parameters, or if confirmation is needed (even with empty params) -->
				{#if hasParameters || message.needsConfirmation}
					<div class={message.needsConfirmation ? 'opacity-80' : ''}>
						<ToolContentDisplay
							title="Parameters"
							content={message.parameters}
							streaming={message.isStreamingArguments}
							toolName={message.toolName}
							showFade={message.showFade}
						/>
					</div>
				{/if}

				<!-- Confirmation Footer -->
				{#if message.needsConfirmation}
					<div
						class={twMerge(
							'mt-3 pt-3 flex flex-row items-center justify-end gap-2',
							hasParameters ? 'border-t border-gray-200 dark:border-gray-700' : ''
						)}
					>
						<Button
							variant="default"
							size="xs"
							on:click={() => {
								if (message.tool_call_id) {
									aiChatManager.handleToolConfirmation(message.tool_call_id, false)
								}
							}}
							startIcon={{ icon: XCircle }}
							destructive
						></Button>
						<Button
							variant="accent"
							size="xs"
							on:click={() => {
								if (message.tool_call_id) {
									aiChatManager.handleToolConfirmation(message.tool_call_id, true)
								}
							}}
							startIcon={{ icon: Play }}
						>
							Run
						</Button>
					</div>

					<!-- Logs and Result - hide while streaming -->
				{:else if !message.isStreamingArguments}
					<ToolContentDisplay
						title="Logs"
						content={message.logs}
						loading={message.isLoading}
						showWhileLoading={false}
					/>

					<ToolContentDisplay
						title="Result"
						content={message.result}
						error={message.error}
						loading={message.isLoading}
					/>
				{/if}
			</div>
		{/if}
	</div>

	{#if previewInfo}
		{@const PreviewIcon = previewInfo.icon}
		{@const clickable = previewInfo.kind === 'schedule'}
		<button
			type="button"
			disabled={!clickable}
			onclick={() => {
				if (!clickable) return
				openPreviewModal({
					kind: previewInfo.kind,
					title: previewInfo.title
				})
			}}
			class={twMerge(
				'flex items-center gap-3 p-2.5 bg-surface border border-gray-200 dark:border-gray-700 rounded-md font-sans w-full text-left',
				clickable
					? 'hover:bg-surface-hover hover:border-gray-300 dark:hover:border-gray-600 transition-colors cursor-pointer'
					: 'cursor-default'
			)}
		>
			<div
				class={twMerge(
					'w-8 h-8 rounded-md flex items-center justify-center shrink-0',
					previewInfo.language ? 'bg-surface-secondary' : previewInfo.iconBg
				)}
			>
				{#if previewInfo.language}
					<LanguageIcon lang={previewInfo.language as any} width={18} height={18} />
				{:else}
					<PreviewIcon class={twMerge('w-4 h-4', previewInfo.iconColor)} />
				{/if}
			</div>
			<div class="flex flex-col flex-1 min-w-0">
				<span class="text-xs font-medium text-primary truncate">{previewInfo.title}</span>
				<span class="text-2xs text-secondary truncate">{previewInfo.subtitle}</span>
			</div>
			<!-- The Open button intentionally stops propagation so the card click handler
			     (modal) and the Open click handler (full-page nav, TBD) stay independent. -->
			<span
				role="button"
				tabindex="0"
				class="inline-flex items-center gap-1 px-2.5 py-1 text-xs rounded-md border border-gray-200 dark:border-gray-700 hover:bg-surface-hover transition-colors text-primary"
				onclick={(e) => {
					e.stopPropagation()
					// TODO: navigate to the resource page
				}}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault()
						e.stopPropagation()
					}
				}}
			>
				<ArrowUpRight class="w-3.5 h-3.5" />
				Open
			</span>
		</button>
	{/if}
</div>
