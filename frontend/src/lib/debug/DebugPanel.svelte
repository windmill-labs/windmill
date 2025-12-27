<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity'
	import { ChevronRight, ChevronDown, Layers, Variable, Terminal } from 'lucide-svelte'
	import type { StackFrame, Scope, Variable as VariableType, DAPClient } from './dapClient'

	interface Props {
		stackFrames: StackFrame[]
		scopes: Scope[]
		variables: Map<number, VariableType[]>
		output: string[]
		client: DAPClient | null
	}

	let { stackFrames, scopes, variables, output, client }: Props = $props()

	let activeTab: 'variables' | 'callstack' | 'output' = $state('variables')
	let expandedScopes = new SvelteSet<number>()
	let selectedFrameId: number | null = $state(null)

	async function toggleScope(scopeRef: number): Promise<void> {
		if (expandedScopes.has(scopeRef)) {
			expandedScopes.delete(scopeRef)
		} else {
			expandedScopes.add(scopeRef)

			// Fetch variables for this scope if not already fetched
			if (!variables.has(scopeRef) && client) {
				await client.getVariables(scopeRef)
			}
		}
	}

	async function selectFrame(frame: StackFrame): Promise<void> {
		selectedFrameId = frame.id
		if (client) {
			await client.getScopes(frame.id)
		}
	}

	function formatValue(value: string): string {
		if (value.length > 50) {
			return value.substring(0, 47) + '...'
		}
		return value
	}
</script>

<div class="flex flex-col h-full bg-surface border-t border-surface-secondary">
	<!-- Tabs -->
	<div class="flex border-b border-surface-secondary">
		<button
			class="px-3 py-1.5 text-xs font-medium flex items-center gap-1 border-b-2 transition-colors"
			class:border-blue-500={activeTab === 'variables'}
			class:text-primary={activeTab === 'variables'}
			class:border-transparent={activeTab !== 'variables'}
			class:text-secondary={activeTab !== 'variables'}
			onclick={() => (activeTab = 'variables')}
		>
			<Variable size={14} />
			Variables
		</button>
		<button
			class="px-3 py-1.5 text-xs font-medium flex items-center gap-1 border-b-2 transition-colors"
			class:border-blue-500={activeTab === 'callstack'}
			class:text-primary={activeTab === 'callstack'}
			class:border-transparent={activeTab !== 'callstack'}
			class:text-secondary={activeTab !== 'callstack'}
			onclick={() => (activeTab = 'callstack')}
		>
			<Layers size={14} />
			Call Stack
		</button>
		<button
			class="px-3 py-1.5 text-xs font-medium flex items-center gap-1 border-b-2 transition-colors"
			class:border-blue-500={activeTab === 'output'}
			class:text-primary={activeTab === 'output'}
			class:border-transparent={activeTab !== 'output'}
			class:text-secondary={activeTab !== 'output'}
			onclick={() => (activeTab = 'output')}
		>
			<Terminal size={14} />
			Output
			{#if output.length > 0}
				<span class="bg-blue-500 text-white text-xs px-1.5 rounded-full">{output.length}</span>
			{/if}
		</button>
	</div>

	<!-- Content -->
	<div class="flex-1 overflow-auto p-2">
		{#if activeTab === 'variables'}
			{#if scopes.length === 0}
				<div class="text-xs text-tertiary italic">No variables to display</div>
			{:else}
				{#each scopes as scope (scope.variablesReference)}
					<div class="mb-2">
						<button
							class="flex items-center gap-1 text-xs font-medium text-secondary hover:text-primary w-full text-left py-1"
							onclick={() => toggleScope(scope.variablesReference)}
						>
							{#if expandedScopes.has(scope.variablesReference)}
								<ChevronDown size={14} />
							{:else}
								<ChevronRight size={14} />
							{/if}
							{scope.name}
						</button>

						{#if expandedScopes.has(scope.variablesReference)}
							<div class="ml-4 space-y-0.5">
								{#each variables.get(scope.variablesReference) || [] as variable (variable.name)}
									<div class="flex items-center text-xs py-0.5 font-mono">
										<span class="text-blue-500 mr-2">{variable.name}</span>
										<span class="text-tertiary mr-1">=</span>
										<span class="text-primary" title={variable.value}>
											{formatValue(variable.value)}
										</span>
										{#if variable.type}
											<span class="text-tertiary ml-2 text-[10px]">({variable.type})</span>
										{/if}
									</div>
								{:else}
									<div class="text-xs text-tertiary italic">No variables</div>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			{/if}

		{:else if activeTab === 'callstack'}
			{#if stackFrames.length === 0}
				<div class="text-xs text-tertiary italic">No call stack available</div>
			{:else}
				<div class="space-y-0.5">
					{#each stackFrames as frame (frame.id)}
						<button
							class="w-full text-left px-2 py-1 text-xs rounded hover:bg-surface-hover transition-colors"
							class:bg-surface-selected={selectedFrameId === frame.id}
							onclick={() => selectFrame(frame)}
						>
							<div class="font-medium text-primary">{frame.name}</div>
							<div class="text-tertiary text-[10px]">
								{frame.source?.name || 'unknown'}:{frame.line}
							</div>
						</button>
					{/each}
				</div>
			{/if}

		{:else if activeTab === 'output'}
			{#if output.length === 0}
				<div class="text-xs text-tertiary italic">No output yet</div>
			{:else}
				<div class="font-mono text-xs space-y-0.5">
					{#each output as line, idx (idx)}
						<div class="text-primary whitespace-pre-wrap">{line}</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>
