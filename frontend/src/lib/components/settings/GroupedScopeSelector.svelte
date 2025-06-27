<script lang="ts">
	import { TokenService, type GroupedScopesResponse } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import ScopeGroup from './ScopeGroup.svelte'

	interface Props {
		selectedScopes?: string[]
		disabled?: boolean
		class?: string
	}

	let { selectedScopes = $bindable([]), disabled = false, class: className = '' }: Props = $props()

	let groupedScopes = $state<GroupedScopesResponse | null>(null)
	let loading = $state(false)
	let error = $state<string | null>(null)

	async function fetchGroupedScopes(): Promise<void> {
		loading = true
		error = null

		try {
			groupedScopes = await TokenService.listGroupedTokenScopes()
		} catch (err) {
			console.error('Error fetching grouped scopes:', err)
			sendUserToast('Failed to load scope options', true)
		} finally {
			loading = false
		}
	}

	function handleScopeChange(scope: string, selected: boolean, resourcePath?: string) {
		if (selected) {
			const scopeString = resourcePath ? `${scope}:${resourcePath}` : scope

			selectedScopes = selectedScopes.filter((s) => !s.startsWith(scope + ':') && s !== scope)

			selectedScopes = [...selectedScopes, scopeString]
		} else {
			selectedScopes = selectedScopes.filter((s) => !s.startsWith(scope + ':') && s !== scope)
		}
	}

	function selectAllScopes() {
		if (!groupedScopes) return

		const allScopes: string[] = []

		for (const group of groupedScopes.groups) {
			for (const scope of group.scopes) {
				if (scope.requires_resource_path) {
					allScopes.push(`${scope.value}:*`)
				} else {
					allScopes.push(scope.value)
				}
			}
		}

		selectedScopes = allScopes
	}

	function clearAllScopes() {
		selectedScopes = []
	}

	function selectAdministratorScope() {
		selectedScopes = ['*']
	}

	const hasAdministratorScope = $derived(selectedScopes.includes('*'))
	fetchGroupedScopes()
</script>

<div class={`${className}`}>
	{#if loading}
		<div class="flex items-center justify-center p-8">
			<div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
			<span class="ml-3 text-sm text-tertiary">Loading scope options...</span>
		</div>
	{:else if error}
		<div class="p-3 bg-red-50 border border-red-200 rounded text-red-800">
			<p class="text-sm">{error}</p>
			<button
				onclick={fetchGroupedScopes}
				class="mt-2 text-sm text-red-700 underline hover:no-underline"
			>
				Try again
			</button>
		</div>
	{:else if groupedScopes}
		<!-- Quick actions -->
		<div class="mb-4 p-3 bg-surface border rounded">
			<h4 class="text-sm font-medium text-primary mb-2">Quick Actions</h4>
			<div class="flex flex-wrap gap-2">
				<button
					onclick={selectAdministratorScope}
					{disabled}
					class="px-2 py-1 text-xs border rounded transition-colors duration-150 {disabled
						? 'opacity-50 cursor-not-allowed'
						: 'hover:bg-surface-hover'} bg-red-50 text-red-700 border-red-200"
				>
					Administrator (Full Access)
				</button>
				<button
					onclick={selectAllScopes}
					{disabled}
					class="px-2 py-1 text-xs border rounded transition-colors duration-150 {disabled
						? 'opacity-50 cursor-not-allowed'
						: 'hover:bg-surface-hover'} bg-blue-50 text-blue-700 border-blue-200"
				>
					Select All Scopes
				</button>
				<button
					onclick={clearAllScopes}
					{disabled}
					class="px-2 py-1 text-xs border rounded transition-colors duration-150 {disabled
						? 'opacity-50 cursor-not-allowed'
						: 'hover:bg-surface-hover'} bg-surface-secondary text-secondary"
				>
					Clear All
				</button>
			</div>

			{#if hasAdministratorScope}
				<div class="mt-3 p-2 bg-yellow-50 border border-yellow-200 rounded text-xs text-yellow-800">
					<span class="font-medium">⚠️ Administrator scope</span> grants full access and overrides all
					other scope selections.
				</div>
			{/if}
		</div>

		<div class="mb-4 p-3 bg-surface-secondary border rounded">
			<h4 class="text-sm font-medium text-primary mb-2">
				Selected Scopes ({selectedScopes.length})
			</h4>
			{#if selectedScopes.length === 0}
				<p class="text-xs text-tertiary">No scopes selected. Token will have full access.</p>
			{:else if hasAdministratorScope}
				<p class="text-xs text-tertiary">Administrator scope grants full access to all resources.</p
				>
			{:else}
				<div class="flex flex-wrap gap-1">
					{#each selectedScopes.slice(0, 10) as scope}
						<span
							class="inline-block px-2 py-0.5 text-xs bg-blue-100 text-blue-800 rounded font-mono"
						>
							{scope}
						</span>
					{/each}
					{#if selectedScopes.length > 10}
						<span class="inline-block px-2 py-0.5 text-xs bg-surface text-secondary rounded">
							+{selectedScopes.length - 10} more
						</span>
					{/if}
				</div>
			{/if}
		</div>

		<div class="border rounded bg-surface max-h-96 overflow-y-auto p-2">
			{#each groupedScopes.groups as group}
				<ScopeGroup {group} {selectedScopes} {disabled} onScopeChange={handleScopeChange} />
			{/each}
		</div>
	{/if}
</div>
