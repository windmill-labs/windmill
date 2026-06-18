<script lang="ts">
	import { Search, Layers, Variable } from 'lucide-svelte'
	import type { StackFrame, Scope, Variable as VariableType, DAPClient } from './dapClient'
	import DebugVariableViewer from './DebugVariableViewer.svelte'

	interface Props {
		stackFrames: StackFrame[]
		scopes: Scope[]
		variables: Map<number, VariableType[]>
		client: DAPClient | null
		selectedFrameId?: number | null
	}

	let { stackFrames, scopes, variables, client, selectedFrameId = $bindable(null) }: Props =
		$props()

	let searchQuery = $state('')

	// Auto-expand scopes when they become available
	$effect(() => {
		console.log('[DebugPanel] effect running, scopes:', scopes, 'variables:', variables)
		for (const scope of scopes) {
			console.log(
				'[DebugPanel] checking scope:',
				scope.name,
				'ref:',
				scope.variablesReference,
				'has:',
				variables.has(scope.variablesReference)
			)
			if (!variables.has(scope.variablesReference) && client) {
				console.log('[DebugPanel] fetching variables for scope:', scope.name)
				client.getVariables(scope.variablesReference)
			}
		}
	})

	async function selectFrame(frame: StackFrame): Promise<void> {
		selectedFrameId = frame.id
		if (client) {
			await client.getScopes(frame.id)
		}
	}

	// Get all variables from all scopes, filtered by search query
	const filteredVariables = $derived.by(() => {
		const allVars: { scope: string; variable: VariableType }[] = []
		for (const scope of scopes) {
			const scopeVars = variables.get(scope.variablesReference) || []
			for (const v of scopeVars) {
				if (!searchQuery || v.name.toLowerCase().includes(searchQuery.toLowerCase())) {
					allVars.push({ scope: scope.name, variable: v })
				}
			}
		}
		return allVars
	})
</script>

<div class="flex h-full bg-surface border-t border-surface-secondary">
	<!-- Variables Panel -->
	<div class="flex-1 flex flex-col border-r border-surface-secondary min-w-0">
		<div
			class="flex items-center gap-1 px-2 py-1 border-b border-surface-secondary bg-surface-secondary"
		>
			<Variable size={12} class="text-secondary" />
			<span class="text-xs font-medium text-secondary">Variables</span>
		</div>
		<div class="px-1.5 py-1 border-b border-surface-secondary">
			<div
				class="flex items-center gap-1.5 px-1.5 py-0.5 bg-surface border border-surface-secondary rounded focus-within:border-blue-500"
			>
				<Search size={12} class="text-tertiary flex-shrink-0" />
				<input
					type="text"
					placeholder="Filter..."
					bind:value={searchQuery}
					class="flex-1 text-xs bg-transparent focus:outline-none min-w-0"
				/>
			</div>
		</div>
		<div class="flex-1 overflow-auto p-1">
			{#if scopes.length === 0}
				<div class="text-xs text-tertiary italic px-1">No variables</div>
			{:else if filteredVariables.length === 0}
				<div class="text-xs text-tertiary italic px-1">No matches</div>
			{:else}
				{#each filteredVariables as { scope, variable } (scope + variable.name)}
					<DebugVariableViewer {variable} {client} />
				{/each}
			{/if}
		</div>
	</div>

	<!-- Call Stack Panel -->
	<div class="w-40 flex flex-col min-w-0">
		<div
			class="flex items-center gap-1 px-2 py-1 border-b border-surface-secondary bg-surface-secondary"
		>
			<Layers size={12} class="text-secondary" />
			<span class="text-xs font-medium text-secondary">Call Stack</span>
		</div>
		<div class="flex-1 overflow-auto p-1">
			{#if stackFrames.length === 0}
				<div class="text-xs text-tertiary italic px-1">No call stack</div>
			{:else}
				{#each stackFrames as frame (frame.id)}
					<button
						class="w-full text-left px-1 py-0.5 text-xs hover:bg-surface-hover rounded flex items-center gap-1 font-mono"
						class:bg-surface-selected={selectedFrameId === frame.id}
						onclick={() => selectFrame(frame)}
					>
						<span class="text-primary font-medium truncate">{frame.name}</span>
						<span class="text-tertiary text-[10px] whitespace-nowrap">:{frame.line}</span>
					</button>
				{/each}
			{/if}
		</div>
	</div>
</div>
