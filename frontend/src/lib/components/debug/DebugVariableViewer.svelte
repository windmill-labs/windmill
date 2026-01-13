<script lang="ts">
	import { ChevronRight, ChevronDown } from 'lucide-svelte'
	import type { Variable, DAPClient } from './dapClient'
	import DebugVariableViewer from './DebugVariableViewer.svelte'

	interface Props {
		variable: Variable
		client: DAPClient | null
		level?: number
		prefix?: string
	}

	let { variable, client, level = 0, prefix = '' }: Props = $props()

	let expanded = $state(false)
	let children: Variable[] = $state([])
	let loading = $state(false)
	let loaded = $state(false)

	const hasChildren = $derived(variable.variablesReference > 0)
	const isExpandable = $derived(hasChildren)

	async function toggleExpand(): Promise<void> {
		if (!isExpandable) return

		if (!expanded && !loaded && client) {
			loading = true
			try {
				children = await client.getVariables(variable.variablesReference)
				loaded = true
			} catch (error) {
				console.error('Failed to fetch nested variables:', error)
			} finally {
				loading = false
			}
		}
		expanded = !expanded
	}

	function getTypeColor(type: string | undefined): string {
		switch (type) {
			case 'string':
				return 'text-green-600 dark:text-green-400/80'
			case 'number':
				return 'text-orange-600 dark:text-orange-400/90'
			case 'boolean':
				return 'text-blue-600 dark:text-blue-400/90'
			case 'undefined':
			case 'null':
				return 'text-tertiary'
			case 'function':
				return 'text-purple-600 dark:text-purple-400/90'
			default:
				return 'text-primary'
		}
	}

	function formatValue(value: string, type: string | undefined): string {
		// For objects/arrays with children, just show type indicator
		if (hasChildren) {
			if (type === 'object' || value.startsWith('{')) {
				return '{...}'
			}
			if (value.startsWith('[')) {
				return '[...]'
			}
		}
		return value
	}
</script>

<div class="font-mono text-xs" style="padding-left: {level * 12}px">
	<div class="flex items-start gap-0.5 py-0.5 hover:bg-surface-hover rounded group">
		{#if isExpandable}
			<button
				class="flex-shrink-0 p-0.5 hover:bg-surface-secondary rounded"
				onclick={toggleExpand}
			>
				{#if loading}
					<span class="inline-block w-3 h-3 border border-tertiary border-t-transparent rounded-full animate-spin"></span>
				{:else if expanded}
					<ChevronDown size={12} class="text-tertiary" />
				{:else}
					<ChevronRight size={12} class="text-tertiary" />
				{/if}
			</button>
		{:else}
			<span class="w-4 flex-shrink-0"></span>
		{/if}

		<div class="flex-1 flex flex-wrap items-baseline gap-1 min-w-0">
			<span class="text-blue-500 font-medium">{variable.name}</span>
			<span class="text-tertiary">=</span>
			<span class={getTypeColor(variable.type)} title={variable.value}>
				{#if hasChildren && expanded}
					{variable.type === 'object' || variable.value.startsWith('{') ? '{' : '['}
				{:else}
					{formatValue(variable.value, variable.type)}
				{/if}
			</span>
			{#if variable.type && !hasChildren}
				<span class="text-tertiary text-[10px]">({variable.type})</span>
			{/if}
		</div>
	</div>

	{#if expanded && hasChildren}
		<div class="border-l border-dotted border-surface-secondary ml-2">
			{#if children.length === 0 && loaded}
				<div class="text-tertiary italic py-0.5" style="padding-left: {12}px">
					No properties
				</div>
			{:else}
				{#each children as child (child.name)}
					<DebugVariableViewer
						variable={child}
						{client}
						level={level + 1}
						prefix={prefix ? `${prefix}.${variable.name}` : variable.name}
					/>
				{/each}
			{/if}
		</div>
		<div class="text-primary" style="padding-left: {(level + 1) * 12}px">
			{variable.type === 'object' || variable.value.startsWith('{') ? '}' : ']'}
		</div>
	{/if}
</div>
