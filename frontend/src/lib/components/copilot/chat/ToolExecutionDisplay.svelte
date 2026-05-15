<script lang="ts">
	import {
		Loader2,
		ChevronDown,
		ChevronRight,
		XCircle,
		Play,
		ExternalLink,
		PanelRight
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { ToolDisplayMessage } from './shared'
	import { twMerge } from 'tailwind-merge'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import ToolMessageActions from './ToolMessageActions.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		extractCandidatePaths,
		hasInlineDrawer,
		itemHref,
		workspaceItemRegistry,
		type WorkspaceItemEntry
	} from './workspaceItems.svelte'

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

	const visibleActions = $derived(
		message.actions && !message.isLoading && !message.error && !message.needsConfirmation
			? message.actions
			: []
	)

	$effect(() => {
		const ws = $workspaceStore
		if (ws) workspaceItemRegistry.ensureLoaded(ws)
	})

	/**
	 * Resolve workspace item paths referenced by the tool's parameters.
	 *
	 * Looks at direct string fields (`path`, `script_path`, `flow_path`, `app_path`) first —
	 * those are the canonical shapes used by tool schemas. Falls back to a regex sweep over
	 * the stringified parameters to catch tools that embed paths in less obvious fields.
	 */
	const referencedItems = $derived.by((): WorkspaceItemEntry[] => {
		const ws = $workspaceStore
		if (!ws) return []
		// Touch reactive state so we re-run when the registry populates.
		workspaceItemRegistry.isLoaded(ws)
		const params = message.parameters
		if (!params || typeof params !== 'object') return []

		const candidates = new Set<string>()
		const PATH_KEYS = ['path', 'script_path', 'flow_path', 'app_path', 'runnable_path']
		for (const key of PATH_KEYS) {
			const value = (params as Record<string, unknown>)[key]
			if (typeof value === 'string' && value.length > 0) {
				candidates.add(value)
			}
		}
		// Fallback: sweep stringified params for any path-shaped tokens we may have missed.
		try {
			const stringified = JSON.stringify(params)
			for (const candidate of extractCandidatePaths(stringified)) {
				candidates.add(candidate)
			}
		} catch {}

		const resolved: WorkspaceItemEntry[] = []
		const seen = new Set<string>()
		for (const candidate of candidates) {
			if (seen.has(candidate)) continue
			const entry = workspaceItemRegistry.resolve(ws, candidate)
			if (entry) {
				resolved.push(entry)
				seen.add(candidate)
			}
		}
		return resolved
	})
</script>

<div
	class="bg-surface border border-gray-200 dark:border-gray-700 rounded-md overflow-hidden font-mono text-xs"
>
	<!-- Collapsible Header -->
	<div
		class={twMerge(
			'border-b border-gray-200 dark:border-gray-700 bg-surface-secondary',
			message.needsConfirmation ? 'opacity-80' : ''
		)}
	>
		<button
			class="w-full p-2 hover:bg-surface-hover transition-colors flex items-start gap-2 text-left"
			onclick={() => (isExpanded = !isExpanded)}
			disabled={!message.showDetails && !message.isStreamingArguments}
		>
			{#if message.showDetails || message.isStreamingArguments}
				<span class="shrink-0 mt-0.5">
					{#if isExpanded}
						<ChevronDown class="w-3 h-3 text-secondary" />
					{:else}
						<ChevronRight class="w-3 h-3 text-secondary" />
					{/if}
				</span>
			{/if}

			<span class="shrink-0 mt-0.5">
				{#if message.isLoading && !message.needsConfirmation}
					<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
				{:else if message.error}
					<span class="text-red-500">✗</span>
				{:else if !message.isLoading && !message.error}
					<span class="text-green-500">✓</span>
				{/if}
			</span>
			<span class="text-primary font-medium text-2xs flex-1 min-w-0">
				{message.content}
			</span>
		</button>
		{#if referencedItems.length > 0}
			<!-- Chip row lives outside the toggle button so we can include real <button>s
				 for the "open in drawer" affordance without nesting interactive elements. -->
			<div class="flex flex-row flex-wrap items-center gap-1 px-2 pb-2 -mt-1 min-w-0">
				{#each referencedItems as item (item.path)}
					<span class="group inline-flex items-center min-w-0 max-w-full">
						<a
							href={itemHref(item, $workspaceStore ?? undefined)}
							target="_blank"
							rel="noopener noreferrer"
							title={item.summary || item.path}
							class="inline-flex items-center gap-1 px-1 py-0.5 rounded hover:bg-surface-hover text-primary no-underline font-mono text-2xs min-w-0"
						>
							<span class="inline-flex shrink-0">
								<RowIcon kind={item.kind} size={12} />
							</span>
							<span class="truncate">{item.path}</span>
							<ExternalLink
								class="w-2.5 h-2.5 shrink-0 text-tertiary opacity-0 group-hover:opacity-100 transition-opacity"
							/>
						</a>
						{#if hasInlineDrawer(item.kind)}
							<button
								type="button"
								onclick={() =>
									aiChatManager.toggleWorkspaceItemDrawer({
										kind: item.kind,
										path: item.path
									})}
								title="Open in drawer"
								aria-label="Open {item.path} in drawer"
								class="ml-0.5 inline-flex self-center shrink-0 rounded p-0.5 text-tertiary hover:bg-surface-hover opacity-0 group-hover:opacity-100 transition-opacity"
							>
								<PanelRight class="w-2.5 h-2.5" />
							</button>
						{/if}
					</span>
				{/each}
			</div>
		{/if}
	</div>

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

				{#if visibleActions.length > 0}
					<ToolMessageActions actions={visibleActions} />
				{:else}
					<ToolContentDisplay
						title="Result"
						content={message.result}
						error={message.error}
						loading={message.isLoading}
					/>
				{/if}
			{/if}
		</div>
	{/if}
</div>
