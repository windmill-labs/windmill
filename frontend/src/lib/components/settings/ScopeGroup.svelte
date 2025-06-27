<script lang="ts">
	import type { ScopeGroup as ScopeGroupType, ScopeDefinition } from '$lib/gen'

	interface Props {
		group: ScopeGroupType
		selectedScopes?: string[]
		disabled?: boolean
		onScopeChange?: (scope: string, selected: boolean, resourcePath?: string) => void
	}

	let { group, selectedScopes = [], disabled = false, onScopeChange }: Props = $props()

	let scopeStates = $state<Map<string, boolean>>(new Map())
	let resourcePaths = $state<Map<string, string>>(new Map())

	// Initialize scope states based on selectedScopes
	$effect(() => {
		const newScopeStates = new Map<string, boolean>()
		const newResourcePaths = new Map<string, string>()
		
		for (const scope of group.scopes) {
			const isSelected = selectedScopes.some(selected => {
				// Check if this is a resource-scoped permission
				if (scope.requires_resource_path && selected.startsWith(scope.value + ':')) {
					const resourcePath = selected.substring(scope.value.length + 1)
					newResourcePaths.set(scope.value, resourcePath)
					return true
				}
				return selected === scope.value
			})
			newScopeStates.set(scope.value, isSelected)
		}
		
		scopeStates = newScopeStates
		resourcePaths = newResourcePaths
	})

	function handleScopeToggle(scope: ScopeDefinition, checked: boolean) {
		scopeStates.set(scope.value, checked)
		
		if (checked && scope.requires_resource_path) {
			// If no resource path is set, provide a default
			if (!resourcePaths.get(scope.value)) {
				resourcePaths.set(scope.value, '*')
			}
		}
		
		const resourcePath = scope.requires_resource_path ? resourcePaths.get(scope.value) : undefined
		onScopeChange?.(scope.value, checked, resourcePath)
	}

	function handleResourcePathChange(scope: ScopeDefinition, resourcePath: string) {
		resourcePaths.set(scope.value, resourcePath)
		
		if (scopeStates.get(scope.value)) {
			onScopeChange?.(scope.value, true, resourcePath)
		}
	}

	function getScopeDisplayName(scope: ScopeDefinition): string {
		return scope.label
	}

	function getScopeDescription(scope: ScopeDefinition): string {
		return scope.description || ''
	}
</script>

<div class="border rounded p-4 mb-4 bg-surface-secondary">
	<div class="mb-3">
		<h3 class="text-base font-semibold text-primary">{group.name}</h3>
		{#if group.description}
			<p class="text-sm text-tertiary mt-1">{group.description}</p>
		{/if}
	</div>

	<div class="space-y-2">
		{#each group.scopes as scope}
			{@const isSelected = scopeStates.get(scope.value) || false}
			{@const resourcePath = resourcePaths.get(scope.value) || ''}
			
			<div class="border rounded p-3 bg-surface transition-colors duration-150 {isSelected ? 'bg-surface-selected border-blue-500' : 'hover:bg-surface-hover'}">
				<div class="flex items-start gap-3">
					<div class="flex-shrink-0 mt-0.5">
						<input
							type="checkbox"
							id={`scope-${scope.value}`}
							checked={isSelected}
							{disabled}
							onchange={(e) => handleScopeToggle(scope, e.currentTarget.checked)}
							class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded {disabled ? 'opacity-50 cursor-not-allowed' : ''}"
						/>
					</div>
					
					<div class="flex-1 min-w-0">
						<label 
							for={`scope-${scope.value}`}
							class="block text-sm font-medium text-primary {disabled ? 'cursor-not-allowed' : 'cursor-pointer'}"
						>
							{getScopeDisplayName(scope)}
						</label>
						
						{#if getScopeDescription(scope)}
							<p class="text-xs text-tertiary mt-1">
								{getScopeDescription(scope)}
							</p>
						{/if}
						
						<div class="text-xs text-secondary mt-1 font-mono bg-surface-secondary px-1.5 py-0.5 rounded inline-block">
							{scope.value}
						</div>

						{#if scope.requires_resource_path && isSelected}
							<div class="mt-3 pl-1">
								<label for={`resource-${scope.value}`} class="block text-xs font-medium text-secondary mb-1">
									Resource Path
								</label>
								<input
									type="text"
									id={`resource-${scope.value}`}
									value={resourcePath}
									placeholder="e.g., f/folder/*, script_name, *"
									{disabled}
									oninput={(e) => handleResourcePathChange(scope, e.currentTarget.value)}
									class="w-full text-xs px-2 py-1.5 border rounded focus:ring-1 focus:ring-blue-500 focus:border-blue-500 bg-surface"
								/>
								<p class="text-xs text-tertiary mt-1">
									Specify resource path. Use '*' for all resources, 'f/folder/*' for folder contents, or specific paths.
								</p>
							</div>
						{/if}
					</div>
				</div>
			</div>
		{/each}
	</div>
</div>

