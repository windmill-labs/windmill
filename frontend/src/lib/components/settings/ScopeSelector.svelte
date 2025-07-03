<script lang="ts">
	import { type ScopeDomain, type ScopeDefinition, TokenService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, ChevronDown, ChevronRight, Pen } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import Popover from '../meltComponents/Popover.svelte'

	interface Props {
		selectedScopes?: string[]
		disabled?: boolean
		class?: string
	}

	interface ScopeState {
		isSelected: boolean
		resourcePaths: string[]
		currentInputValue: string
		pathError?: string
	}

	interface DomainState {
		isExpanded: boolean
		hasFullAccess: boolean
		scopes: Record<string, ScopeState>
	}

	interface ComponentState {
		domains: Record<string, DomainState>
	}

	let { selectedScopes = $bindable([]), disabled = false, class: className = '' }: Props = $props()

	let scopeDomains = $state<ScopeDomain[] | null>(null)
	let loading = $state(false)
	let error = $state<string | null>(null)

	let componentState = $state<ComponentState>({ domains: {} })

	function createScopeState(scope: ScopeDefinition): ScopeState {
		return {
			isSelected: false,
			resourcePaths: [],
			currentInputValue: '',
			pathError: undefined
		}
	}

	function createDomainState(domain: ScopeDomain): DomainState {
		const scopes: Record<string, ScopeState> = {}
		for (const scope of domain.scopes) {
			scopes[scope.value] = createScopeState(scope)
		}
		return {
			isExpanded: false,
			hasFullAccess: false,
			scopes
		}
	}

	function getScopeState(scopeValue: string): ScopeState | undefined {
		for (const domainState of Object.values(componentState.domains)) {
			if (domainState.scopes[scopeValue]) {
				return domainState.scopes[scopeValue]
			}
		}
		return undefined
	}

	function getDomainState(domainName: string): DomainState | undefined {
		return componentState.domains[domainName]
	}

	async function fetchScopeDomains(): Promise<void> {
		loading = true
		error = null

		try {
			scopeDomains = await TokenService.listAvailableScopes()
			initializeDomainStates()
		} catch (err) {
			console.error('Error fetching scope domains:', err)
			sendUserToast('Failed to load scope options', true)
			error = 'Failed to load scope options'
		} finally {
			loading = false
		}
	}

	function initializeDomainStates() {
		if (!scopeDomains) return

		const newDomains: Record<string, DomainState> = {}

		for (const domain of scopeDomains) {
			const domainState = createDomainState(domain)
			
			// Check if domain has full access
			const writeScopeValue = getWriteScopeForDomain(domain)
			const hasWriteSelected =
				writeScopeValue &&
				selectedScopes.some(
					(scope) => scope === writeScopeValue || scope.startsWith(writeScopeValue + ':')
				)
			
			const runScopes = domain.scopes.filter(scope => scope.value.includes(':run:'))
			const hasRunScopesSelected = runScopes.length > 0 && runScopes.every(runScope => 
				selectedScopes.some(scope => scope === runScope.value || scope.startsWith(runScope.value + ':'))
			)
			
			domainState.hasFullAccess = Boolean(hasWriteSelected && (runScopes.length === 0 || hasRunScopesSelected))

			// Initialize individual scope states
			for (const scope of domain.scopes) {
				const scopeState = domainState.scopes[scope.value]
				
				const isSelected = selectedScopes.some((selected) => {
					if (scope.requires_resource_path && selected.startsWith(scope.value + ':')) {
						const resourcePath = selected.substring(scope.value.length + 1)
						const paths = resourcePath.split(',').map((p) => p.trim())
						scopeState.resourcePaths = [...scopeState.resourcePaths, ...paths]
						return true
					}
					return selected === scope.value
				})
				
				scopeState.isSelected = isSelected
			}
			
			newDomains[domain.name] = domainState
		}
		
		componentState = { domains: newDomains }
	}

	function getWriteScopeForDomain(domain: ScopeDomain): string | null {
		const writeScope = domain.scopes.find((scope) => scope.value.endsWith(':write'))
		return writeScope?.value || null
	}

	function toggleDomainExpansion(domainName: string) {
		const domainState = getDomainState(domainName)
		if (domainState) {
			domainState.isExpanded = !domainState.isExpanded
		}
	}

	function handleDomainCheckboxChange(domain: ScopeDomain, checked: boolean) {
		const writeScopeValue = getWriteScopeForDomain(domain)
		if (!writeScopeValue) return

		const domainState = getDomainState(domain.name)
		if (!domainState) return

		domainState.hasFullAccess = checked

		if (checked) {
			// Remove any existing scopes for this domain
			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)

			// Always add the write scope
			selectedScopes = [...selectedScopes, writeScopeValue]
			
			// Also add any run scopes for this domain
			const runScopes = domain.scopes.filter(scope => scope.value.includes(':run:'))
			for (const runScope of runScopes) {
				selectedScopes = [...selectedScopes, runScope.value]
			}
		} else {
			// Remove all scopes for this domain
			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)
		}
	}

	function handleIndividualScopeChange(scope: ScopeDefinition, checked: boolean) {
		const scopeState = getScopeState(scope.value)
		if (!scopeState) return

		scopeState.isSelected = checked

		if (scope.requires_resource_path) {
			if (checked) {
				// Initialize with default wildcard if no paths exist
				if (scopeState.resourcePaths.length === 0) {
					scopeState.resourcePaths = ['*']
					updateSelectedScopesForResourcePaths(scope.value, ['*'])
				}
			} else {
				scopeState.resourcePaths = []
				updateSelectedScopesForResourcePaths(scope.value, [])
			}
		} else {
			if (checked) {
				selectedScopes = selectedScopes.filter(
					(s) => !s.startsWith(scope.value + ':') && s !== scope.value
				)
				selectedScopes = [...selectedScopes, scope.value]
			} else {
				selectedScopes = selectedScopes.filter(
					(s) => !s.startsWith(scope.value + ':') && s !== scope.value
				)
			}
		}

		updateDomainCheckboxState(scope)
	}

	function updateDomainCheckboxState(changedScope: ScopeDefinition) {
		if (!scopeDomains) return

		const domain = scopeDomains.find((d) => d.scopes.some((s) => s.value === changedScope.value))
		if (!domain) return

		const domainState = getDomainState(domain.name)
		if (!domainState) return

		const writeScope = getWriteScopeForDomain(domain)
		const hasWriteSelected = writeScope && domainState.scopes[writeScope]?.isSelected

		const runScopes = domain.scopes.filter(scope => scope.value.includes(':run:'))
		const hasRunScopesSelected = runScopes.length > 0 && runScopes.every(runScope => 
			domainState.scopes[runScope.value]?.isSelected
		)

		const isDomainFullySelected = hasWriteSelected && (runScopes.length === 0 || hasRunScopesSelected)
		domainState.hasFullAccess = Boolean(isDomainFullySelected)
	}

	function getSelectedScopesForDomain(domain: ScopeDomain): string[] {
		const domainState = getDomainState(domain.name)
		if (!domainState) return []

		return domain.scopes
			.filter((scope) => domainState.scopes[scope.value]?.isSelected)
			.map((scope) => {
				const scopeState = domainState.scopes[scope.value]
				const resourcePaths = scopeState?.resourcePaths || []
				return resourcePaths.length > 0
					? `${scope.value}:${resourcePaths.join(',')}`
					: scope.value
			})
	}

	function clearAllScopes() {
		selectedScopes = []
		// Reset all domain states
		for (const domainState of Object.values(componentState.domains)) {
			domainState.hasFullAccess = false
			domainState.isExpanded = false
			for (const scopeState of Object.values(domainState.scopes)) {
				scopeState.isSelected = false
				scopeState.resourcePaths = []
				scopeState.currentInputValue = ''
				scopeState.pathError = undefined
			}
		}
	}

	const hasAdministratorScope = $derived(selectedScopes.includes('*'))

	$effect(() => {
		if (scopeDomains && componentState.domains) {
			syncSelectedScopesWithState()
		}
	})

	function validateResourcePath(path: string): string | null {
		if (!path.trim()) return 'Path cannot be empty'

		const trimmedPath = path.trim()

		if (trimmedPath === '*') return null

		if (trimmedPath === 'u/*' || trimmedPath === 'f/*') return null

		if (!trimmedPath.startsWith('u/') && !trimmedPath.startsWith('f/')) {
			return 'Path must start with u/ or f/'
		}

		const parts = trimmedPath.split('/')
		if (parts.length !== 3) {
			return 'Path must have exactly 3 parts: u/{user}/{resource} or f/{folder}/{resource}'
		}

		if (parts[1] === '') {
			return 'Username/folder name cannot be empty'
		}

		if (parts[2] === '') {
			return 'Resource name cannot be empty'
		}

		if (parts[2] === '*') return null

		if (parts[2].includes('*')) {
			return 'Wildcards can only be used as the full resource name (*)'
		}

		return null
	}

	function addResourcePath(scopeValue: string, path: string) {
		const scopeState = getScopeState(scopeValue)
		if (!scopeState) return false

		const error = validateResourcePath(path)
		if (error) {
			scopeState.pathError = error
			return false
		}

		scopeState.pathError = undefined

		if (scopeState.resourcePaths.includes(path.trim())) {
			scopeState.pathError = 'Path already exists'
			return false
		}

		const newPaths = [...scopeState.resourcePaths, path.trim()]
		scopeState.resourcePaths = newPaths
		scopeState.currentInputValue = ''

		updateSelectedScopesForResourcePaths(scopeValue, newPaths)
		return true
	}

	function removeResourcePath(scopeValue: string, pathToRemove: string) {
		const scopeState = getScopeState(scopeValue)
		if (!scopeState) return

		const newPaths = scopeState.resourcePaths.filter((p) => p !== pathToRemove)
		scopeState.resourcePaths = newPaths
		scopeState.pathError = undefined

		updateSelectedScopesForResourcePaths(scopeValue, newPaths)
	}

	function updateSelectedScopesForResourcePaths(scopeValue: string, paths: string[]) {
		selectedScopes = selectedScopes.filter(
			(s) => !s.startsWith(scopeValue + ':') && s !== scopeValue
		)

		const scopeState = getScopeState(scopeValue)
		if (!scopeState) return

		if (paths.length > 0) {
			const scopeString = `${scopeValue}:${paths.join(',')}`
			selectedScopes = [...selectedScopes, scopeString]
			scopeState.isSelected = true
		} else {
			scopeState.isSelected = false
		}

		updateDomainCheckboxState({ value: scopeValue } as any)
	}

	function syncSelectedScopesWithState() {
		if (!scopeDomains) return

		for (const domain of scopeDomains) {
			const domainState = getDomainState(domain.name)
			if (!domainState) continue

			// Check if domain has full access
			const writeScopeValue = getWriteScopeForDomain(domain)
			const hasWriteSelected =
				writeScopeValue &&
				selectedScopes.some(
					(scope) => scope === writeScopeValue || scope.startsWith(writeScopeValue + ':')
				)
			
			const runScopes = domain.scopes.filter(scope => scope.value.includes(':run:'))
			const hasRunScopesSelected = runScopes.length > 0 && runScopes.every(runScope => 
				selectedScopes.some(scope => scope === runScope.value || scope.startsWith(runScope.value + ':'))
			)
			
			domainState.hasFullAccess = Boolean(hasWriteSelected && (runScopes.length === 0 || hasRunScopesSelected))

			for (const scope of domain.scopes) {
				const scopeState = domainState.scopes[scope.value]
				if (!scopeState) continue

				scopeState.resourcePaths = []
				
				const isSelected = selectedScopes.some((selected) => {
					if (scope.requires_resource_path && selected.startsWith(scope.value + ':')) {
						const resourcePath = selected.substring(scope.value.length + 1)
						const paths = resourcePath.split(',').map((p) => p.trim())
						scopeState.resourcePaths = [...paths]
						return true
					}
					return selected === scope.value
				})
				
				scopeState.isSelected = isSelected
			}
		}
	}

	fetchScopeDomains()
</script>

<div class="w-full {className}">
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<Loader2 size={32} class="animate-spin text-primary" />
		</div>
	{:else if error}
		<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
			<p class="text-sm text-red-800 mb-3">{error}</p>
			<Button onclick={fetchScopeDomains} variant="contained" color="red" size="sm">
				Try again
			</Button>
		</div>
	{:else if scopeDomains}
		<div class="mb-6 p-4 bg-surface-secondary border rounded-lg">
			<div class="flex items-center justify-between mb-3">
				<h4 class="text-sm font-semibold text-primary">
					Selected Scopes ({selectedScopes.length})
				</h4>
				<Button onclick={clearAllScopes} {disabled} variant="border" size="sm" color="red">
					Clear All
				</Button>
			</div>

			{#if selectedScopes.length === 0}
				<p class="text-sm text-tertiary">No scopes selected. Token will have full access.</p>
			{:else if hasAdministratorScope}
				<p class="text-sm text-tertiary">Administrator scope grants full access to all resources.</p
				>
			{:else}
				<div class="flex flex-wrap gap-2">
					{#each selectedScopes.slice(0, 10) as scope}
						<span
							class="inline-flex items-center px-2.5 py-0.5 text-xs font-medium bg-blue-100 text-blue-800 rounded-full font-mono"
						>
							{scope}
						</span>
					{/each}
					{#if selectedScopes.length > 10}
						<span
							class="inline-flex items-center px-2.5 py-0.5 text-xs font-medium bg-surface text-secondary rounded-full"
						>
							+{selectedScopes.length - 10} more
						</span>
					{/if}
				</div>
			{/if}
		</div>

		<div class="space-y-3">
			{#each scopeDomains as domain}
				{@const domainState = getDomainState(domain.name)}
				{@const isExpanded = domainState?.isExpanded || false}
				{@const isDomainSelected = domainState?.hasFullAccess || false}
				{@const selectedScopes = getSelectedScopesForDomain(domain)}

				<div class="border rounded-lg bg-surface overflow-hidden">
					<div class="p-4 bg-surface-secondary border-b">
						<div class="flex items-center gap-3">
							<div class="flex-shrink-0">
								<input
									type="checkbox"
									id={`domain-${domain.name}`}
									checked={isDomainSelected}
									{disabled}
									onchange={(e) => handleDomainCheckboxChange(domain, e.currentTarget.checked)}
									class="w-4 h-4 text-blue-600 bg-surface border-gray-300 rounded focus:ring-blue-500 focus:ring-2"
								/>
							</div>

							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2 flex-wrap">
									<label
										for={`domain-${domain.name}`}
										class="text-sm font-semibold text-primary cursor-pointer"
									>
										{domain.name}
									</label>
									{#each selectedScopes as scope}
										<span
											class="inline-flex items-center px-1.5 py-0.5 text-xs font-medium bg-blue-100 text-blue-800 rounded-full font-mono"
										>
											{scope}
										</span>
									{/each}
								</div>
								{#if domain.description}
									<p class="text-xs text-tertiary mt-0.5">{domain.description}</p>
								{/if}
							</div>

							<Button
								onclick={() => toggleDomainExpansion(domain.name)}
								{disabled}
								size="sm"
								class="p-1"
							>
								{#if isExpanded}
									<ChevronDown size={16} />
								{:else}
									<ChevronRight size={16} />
								{/if}
							</Button>
						</div>
					</div>

					{#if isExpanded}
						<div class="p-2">
							<div class="grid grid-cols-2 gap-4">
								{#each domain.scopes as scope}
									{@const scopeState = domainState?.scopes[scope.value]}
									{@const isSelected = scopeState?.isSelected || false}
									{@const resourcePathArray = scopeState?.resourcePaths || []}
									{@const currentInput = scopeState?.currentInputValue || ''}
									{@const pathError = scopeState?.pathError}

									<div
										class="flex justify-between p-3 border rounded-lg bg-surface-secondary min-h-16 w-full"
									>
										<div class="flex items-center gap-2">
											<input
												type="checkbox"
												id={`scope-${scope.value}`}
												checked={isSelected}
												{disabled}
												onchange={(e) =>
													handleIndividualScopeChange(scope, e.currentTarget.checked)}
												class="!w-4 !h-4"
											/>

											<p class="font-medium text-xs">
												{scope.label}
											</p>
										</div>
										{#if scope.requires_resource_path && isSelected}
											<Popover closeOnOtherPopoverOpen contentClasses="p-3">
												{#snippet trigger()}
													<Button size="xs" variant="border">
														<Pen size={14} />
													</Button>
												{/snippet}
												{#snippet content({ close })}
													<div class="w-80">
														<p class="block text-xs font-medium text-secondary mb-2"> Paths </p>

														<div class="mb-3">
															<input
																type="text"
																value={currentInput}
																placeholder="e.g., u/username/*, f/folder/script.py"
																{disabled}
																oninput={(e) => {
																	if (scopeState) {
																		scopeState.currentInputValue = e.currentTarget.value
																		scopeState.pathError = undefined
																	}
																}}
																onkeydown={(e) => {
																	if (e.key === 'Enter' && currentInput.trim()) {
																		e.preventDefault()
																		addResourcePath(scope.value, currentInput)
																	}
																}}
																class="w-full text-sm px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-surface"
															/>
															<p class="text-xs text-tertiary mt-1">
																Press Enter to add • Formats: u/user/*, f/folder/*, u/user/file,
																f/folder/file, *
															</p>
															{#if pathError}
																<p class="text-xs text-red-600 mt-1">{pathError}</p>
															{/if}
														</div>

														{#if resourcePathArray.length > 0}
															<div class="space-y-2 mb-3 max-h-32 overflow-y-auto">
																{#each resourcePathArray as path}
																	<div
																		class="flex items-center justify-between bg-blue-50 px-2 py-1 rounded text-xs"
																	>
																		<span class="font-mono text-blue-800 flex-1 truncate"
																			>{path}</span
																		>

																		<button
																			type="button"
																			onclick={() => removeResourcePath(scope.value, path)}
																			class="text-red-600 hover:text-red-800 p-0.5"
																			title="Remove path"
																		>
																			×
																		</button>
																	</div>
																{/each}
															</div>
														{/if}

														<div class="flex gap-2 mt-3">
															<Button
																onclick={() => {
																	addResourcePath(scope.value, '*')
																}}
																size="xs"
																variant="border"
																disabled={resourcePathArray.includes('*')}
															>
																Add All (*)
															</Button>
															<Button onclick={close} size="xs">Done</Button>
														</div>
													</div>
												{/snippet}
											</Popover>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
