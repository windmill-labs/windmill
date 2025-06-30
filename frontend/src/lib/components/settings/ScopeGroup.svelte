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
			const isSelected = selectedScopes.some((selected) => {
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
</script>

<div class="border rounded p-2 mb-2 bg-surface-secondary">
	<div class="mb-2">
		<h3 class="text-sm font-semibold text-primary">{group.name}</h3>
	</div>

	<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
		{#each group.scopes as scope}
			{@const isSelected = scopeStates.get(scope.value) || false}
			{@const resourcePath = resourcePaths.get(scope.value) || ''}

			<div class="flex items-start gap-2">
				<div class="flex-shrink-0 mt-0.5">
					<input
						type="checkbox"
						id={`scope-${scope.value}`}
						checked={isSelected}
						{disabled}
						onchange={(e) => handleScopeToggle(scope, e.currentTarget.checked)}
					/>
				</div>

				<div class="flex-1 min-w-0">
					<label for={`scope-${scope.value}`} class="block text-xs font-medium text-primary">
						{getScopeDisplayName(scope)}
					</label>

					{#if scope.requires_resource_path && isSelected}
						<div class="mt-2">
							<label
								for={`resource-${scope.value}`}
								class="block text-xs font-medium text-secondary mb-1"
							>
								Resource Path
							</label>
							<input
								type="text"
								id={`resource-${scope.value}`}
								value={resourcePath}
								placeholder="e.g., f/folder/*, script_name, *"
								{disabled}
								oninput={(e) => handleResourcePathChange(scope, e.currentTarget.value)}
								class="w-full text-xs px-1.5 py-1 border rounded focus:ring-1 focus:ring-blue-500 focus:border-blue-500 bg-surface"
							/>
							<p class="text-xs text-tertiary mt-1 leading-tight">
								Specify resource path. Use '*' for all resources, 'f/folder/*' for folder contents,
								or specific paths.
							</p>
						</div>
					{/if}
				</div>
			</div>
		{/each}
	</div>
</div>
