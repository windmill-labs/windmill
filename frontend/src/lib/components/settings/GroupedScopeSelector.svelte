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

	let { selectedScopes = $bindable([]), disabled = false, class: className = '' }: Props = $props()

	let scopeDomains = $state<ScopeDomain[] | null>(null)
	let loading = $state(false)
	let error = $state<string | null>(null)

	let expandedDomains = $state<Set<string>>(new Set())
	let domainFullAccessSelected = $state<Map<string, boolean>>(new Map())
	let individualScopeSelections = $state<Map<string, boolean>>(new Map())
	let resourcePaths = $state<Record<string, string[]>>({})
	let currentInputValues = $state<Record<string, string>>({})
	let pathErrors = $state<Record<string, string>>({})

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

		const newDomainFullAccessSelected = new Map<string, boolean>()
		const newIndividualScopeSelections = new Map<string, boolean>()
		const newResourcePaths: Record<string, string[]> = {}
		const newCurrentInputValues: Record<string, string> = {}
		const newPathErrors: Record<string, string> = {}

		for (const domain of scopeDomains) {
			const writeScopeValue = getWriteScopeForDomain(domain)
			const hasWriteSelected =
				writeScopeValue &&
				selectedScopes.some(
					(scope) => scope === writeScopeValue || scope.startsWith(writeScopeValue + ':')
				)
			newDomainFullAccessSelected.set(domain.name, Boolean(hasWriteSelected))

			for (const scope of domain.scopes) {
				const isSelected = selectedScopes.some((selected) => {
					if (scope.requires_resource_path && selected.startsWith(scope.value + ':')) {
						const resourcePath = selected.substring(scope.value.length + 1)
						const paths = resourcePath.split(',').map(p => p.trim())
						const existingPaths = newResourcePaths[scope.value] || []
						newResourcePaths[scope.value] = [...existingPaths, ...paths]
						return true
					}
					return selected === scope.value
				})
				newIndividualScopeSelections.set(scope.value, isSelected)
			}
		}
		domainFullAccessSelected = newDomainFullAccessSelected
		individualScopeSelections = newIndividualScopeSelections
		resourcePaths = newResourcePaths
		currentInputValues = newCurrentInputValues
		pathErrors = newPathErrors
	}

	function getWriteScopeForDomain(domain: ScopeDomain): string | null {
		const writeScope = domain.scopes.find((scope) => scope.value.endsWith(':write'))
		return writeScope?.value || null
	}

	function toggleDomainExpansion(domainName: string) {
		const newExpanded = new Set(expandedDomains)
		if (newExpanded.has(domainName)) {
			newExpanded.delete(domainName)
		} else {
			newExpanded.add(domainName)
		}
		expandedDomains = newExpanded
	}

	function handleDomainCheckboxChange(domain: ScopeDomain, checked: boolean) {
		const writeScopeValue = getWriteScopeForDomain(domain)
		if (!writeScopeValue) return

		domainFullAccessSelected.set(domain.name, checked)

		if (checked) {
			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)

			selectedScopes = [...selectedScopes, writeScopeValue]
		} else {
			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)
		}
	}

	function handleIndividualScopeChange(
		scope: ScopeDefinition,
		checked: boolean
	) {
		if (scope.requires_resource_path) {
			individualScopeSelections.set(scope.value, checked)
			if (checked) {
				// Initialize with default wildcard if no paths exist
				const currentPaths = resourcePaths[scope.value] || []
				if (currentPaths.length === 0) {
					resourcePaths[scope.value] = ['*']
					updateSelectedScopesForResourcePaths(scope.value, ['*'])
				}
			} else {
				resourcePaths[scope.value] = []
				updateSelectedScopesForResourcePaths(scope.value, [])
			}
		} else {
			individualScopeSelections.set(scope.value, checked)

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

		const writeScope = getWriteScopeForDomain(domain)
		const hasWriteSelected = writeScope && individualScopeSelections.get(writeScope)

		domainFullAccessSelected.set(domain.name, hasWriteSelected || false)
	}

	function getSelectedScopesForDomain(domain: ScopeDomain): string[] {
		return domain.scopes
			.filter((scope) => individualScopeSelections.get(scope.value))
			.map((scope) => {
				const resourcePathArray = resourcePaths[scope.value]
				return resourcePathArray && resourcePathArray.length > 0 
					? `${scope.value}:${resourcePathArray.join(',')}` 
					: scope.value
			})
	}

	function clearAllScopes() {
		selectedScopes = []
		domainFullAccessSelected.clear()
		individualScopeSelections.clear()
		resourcePaths = {}
		currentInputValues = {}
		pathErrors = {}
		expandedDomains.clear()
		initializeDomainStates()
	}

	const hasAdministratorScope = $derived(selectedScopes.includes('*'))

	// Re-initialize when selectedScopes changes externally
	$effect(() => {
		if (scopeDomains) {
			initializeDomainStates()
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
		const error = validateResourcePath(path)
		if (error) {
			pathErrors[scopeValue] = error
			return false
		}
		
		delete pathErrors[scopeValue]
		const currentPaths = resourcePaths[scopeValue] || []
		
		if (currentPaths.includes(path.trim())) {
			pathErrors[scopeValue] = 'Path already exists'
			return false
		}
		
		if (currentPaths.length >= 10) {
			pathErrors[scopeValue] = 'Maximum 10 paths allowed'
			return false
		}
		
		const newPaths = [...currentPaths, path.trim()]
		resourcePaths[scopeValue] = newPaths
		currentInputValues[scopeValue] = ''
		
		updateSelectedScopesForResourcePaths(scopeValue, newPaths)
		return true
	}

	function removeResourcePath(scopeValue: string, pathToRemove: string) {
		const currentPaths = resourcePaths[scopeValue] || []
		const newPaths = currentPaths.filter(p => p !== pathToRemove)
		resourcePaths[scopeValue] = newPaths
		delete pathErrors[scopeValue]
		
		updateSelectedScopesForResourcePaths(scopeValue, newPaths)
	}

	function editResourcePath(scopeValue: string, oldPath: string) {
		currentInputValues[scopeValue] = oldPath
		removeResourcePath(scopeValue, oldPath)
	}

	function updateSelectedScopesForResourcePaths(scopeValue: string, paths: string[]) {
		selectedScopes = selectedScopes.filter(
			(s) => !s.startsWith(scopeValue + ':') && s !== scopeValue
		)
		
		if (paths.length > 0) {
			const scopeString = `${scopeValue}:${paths.join(',')}`
			selectedScopes = [...selectedScopes, scopeString]
			individualScopeSelections.set(scopeValue, true)
		} else {
			individualScopeSelections.set(scopeValue, false)
		}
		
		updateDomainCheckboxState({ value: scopeValue } as any)
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
				{@const isExpanded = expandedDomains.has(domain.name)}
				{@const isDomainSelected = domainFullAccessSelected.get(domain.name) || false}
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
									{@const isSelected = individualScopeSelections.get(scope.value) || false}
									{@const resourcePathArray = resourcePaths[scope.value] || []}
									{@const currentInput = currentInputValues[scope.value] || ''}
									{@const pathError = pathErrors[scope.value]}

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
													handleIndividualScopeChange(
														scope,
														e.currentTarget.checked
													)}
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
														<p class="block text-xs font-medium text-secondary mb-2">
															{scope.label} Paths ({resourcePathArray.length}/10)
														</p>
														
														<!-- Add new path input -->
														<div class="mb-3">
															<input
																type="text"
																value={currentInput}
																placeholder="e.g., u/username/*, f/folder/script.py"
																{disabled}
																oninput={(e) => {
																	currentInputValues[scope.value] = e.currentTarget.value
																	delete pathErrors[scope.value]
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
																Press Enter to add • Formats: u/user/*, f/folder/*, u/user/file, f/folder/file, *
															</p>
															{#if pathError}
																<p class="text-xs text-red-600 mt-1">{pathError}</p>
															{/if}
														</div>

														<!-- Existing paths as chips -->
														{#if resourcePathArray.length > 0}
															<div class="space-y-2 mb-3 max-h-32 overflow-y-auto">
																{#each resourcePathArray as path}
																	<div class="flex items-center justify-between bg-blue-50 px-2 py-1 rounded text-xs">
																		<span class="font-mono text-blue-800 flex-1 truncate">{path}</span>
																		<div class="flex gap-1 ml-2">
																			<button
																				type="button"
																				onclick={() => editResourcePath(scope.value, path)}
																				class="text-blue-600 hover:text-blue-800 p-0.5"
																				title="Edit path"
																			>
																				<Pen size={12} />
																			</button>
																			<button
																				type="button"
																				onclick={() => removeResourcePath(scope.value, path)}
																				class="text-red-600 hover:text-red-800 p-0.5"
																				title="Remove path"
																			>
																				×
																			</button>
																		</div>
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
