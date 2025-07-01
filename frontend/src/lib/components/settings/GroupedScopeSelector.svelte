<script lang="ts">
	import { type ScopeDomain, type ScopeDefinition, TokenService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, ChevronDown, ChevronRight, Settings } from 'lucide-svelte'
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
	let domainAdminSelected = $state<Map<string, boolean>>(new Map())
	let individualScopeSelections = $state<Map<string, boolean>>(new Map())
	let resourcePaths = $state<Map<string, string>>(new Map())

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

		const newDomainAdminSelected = new Map<string, boolean>()
		const newIndividualScopeSelections = new Map<string, boolean>()
		const newResourcePaths = new Map<string, string>()

		for (const domain of scopeDomains) {
			const writeScopeValue = getWriteScopeForDomain(domain)
			const hasWriteSelected =
				writeScopeValue &&
				selectedScopes.some(
					(scope) => scope === writeScopeValue || scope.startsWith(writeScopeValue + ':')
				)
			newDomainAdminSelected.set(domain.name, Boolean(hasWriteSelected))

			for (const scope of domain.scopes) {
				const isSelected = selectedScopes.some((selected) => {
					if (scope.requires_resource_path && selected.startsWith(scope.value + ':')) {
						const resourcePath = selected.substring(scope.value.length + 1)
						newResourcePaths.set(scope.value, resourcePath)
						return true
					}
					return selected === scope.value
				})
				newIndividualScopeSelections.set(scope.value, isSelected)
			}
		}

		domainAdminSelected = newDomainAdminSelected
		individualScopeSelections = newIndividualScopeSelections
		resourcePaths = newResourcePaths
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

		domainAdminSelected.set(domain.name, checked)

		if (checked) {
			expandedDomains.add(domain.name)

			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)

			selectedScopes = [...selectedScopes, writeScopeValue]

			for (const scope of domain.scopes) {
				individualScopeSelections.set(scope.value, scope.value === writeScopeValue)
			}
		} else {
			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)

			for (const scope of domain.scopes) {
				individualScopeSelections.set(scope.value, false)
			}
		}
	}

	function handleIndividualScopeChange(
		scope: ScopeDefinition,
		checked: boolean,
		resourcePath?: string
	) {
		// Use the generic handler for scopes that require resource paths
		if (scope.requires_resource_path) {
			handleScopeWithResourcePath(scope.value, checked, resourcePath)
		} else {
			// Simple scope without resource path
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

	function handleScopeWithResourcePath(
		scopeValue: string,
		checked: boolean,
		resourcePath?: string
	) {
		individualScopeSelections.set(scopeValue, checked)

		if (checked) {
			const scopeString = resourcePath ? `${scopeValue}:${resourcePath}` : scopeValue

			selectedScopes = selectedScopes.filter(
				(s) => !s.startsWith(scopeValue + ':') && s !== scopeValue
			)

			selectedScopes = [...selectedScopes, scopeString]

			if (!resourcePath) {
				resourcePaths.set(scopeValue, '*')
			}
		} else {
			selectedScopes = selectedScopes.filter(
				(s) => !s.startsWith(scopeValue + ':') && s !== scopeValue
			)
		}
	}

	function handleScopeResourcePathChange(scopeValue: string, resourcePath: string) {
		resourcePaths.set(scopeValue, resourcePath)

		if (individualScopeSelections.get(scopeValue)) {
			handleScopeWithResourcePath(scopeValue, true, resourcePath)
		}
	}

	function updateDomainCheckboxState(changedScope: ScopeDefinition) {
		if (!scopeDomains) return

		const domain = scopeDomains.find((d) => d.scopes.some((s) => s.value === changedScope.value))
		if (!domain) return

		const writeScope = getWriteScopeForDomain(domain)
		const hasWriteSelected = writeScope && individualScopeSelections.get(writeScope)

		domainAdminSelected.set(domain.name, hasWriteSelected || false)
	}

	function getSelectedScopesForDomain(domain: ScopeDomain): string[] {
		if (domain.name === 'Jobs') {
			const jobsScopes = domain.scopes
				.filter((scope) => scope.value !== 'jobs:run' && individualScopeSelections.get(scope.value))
				.map((scope) => {
					const resourcePath = resourcePaths.get(scope.value)
					return resourcePath ? `${scope.value}:${resourcePath}` : scope.value
				})

			const scriptsRunSelected = individualScopeSelections.get('scripts:run')
			const flowsRunSelected = individualScopeSelections.get('flows:run')

			if (scriptsRunSelected) {
				const resourcePath = resourcePaths.get('scripts:run')
				jobsScopes.push(resourcePath ? `scripts:run:${resourcePath}` : 'scripts:run')
			}

			if (flowsRunSelected) {
				const resourcePath = resourcePaths.get('flows:run')
				jobsScopes.push(resourcePath ? `flows:run:${resourcePath}` : 'flows:run')
			}

			return jobsScopes
		}

		return domain.scopes
			.filter((scope) => individualScopeSelections.get(scope.value))
			.map((scope) => {
				const resourcePath = resourcePaths.get(scope.value)
				return resourcePath ? `${scope.value}:${resourcePath}` : scope.value
			})
	}

	function clearAllScopes() {
		selectedScopes = []
		domainAdminSelected.clear()
		individualScopeSelections.clear()
		resourcePaths.clear()
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
		<!-- Selected Scopes Summary -->
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
				{@const isDomainSelected = domainAdminSelected.get(domain.name) || false}
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
							<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
								{#each domain.scopes as scope}
									{@const isSelected = individualScopeSelections.get(scope.value) || false}
									{@const resourcePath = resourcePaths.get(scope.value) || ''}

									{#if domain.name === 'Jobs' && scope.value === 'jobs:run'}
										{@const scriptsRunSelected =
											individualScopeSelections.get('scripts:run') || false}
										{@const flowsRunSelected = individualScopeSelections.get('flows:run') || false}
										{@const scriptsRunResourcePath = resourcePaths.get('scripts:run') || '*'}
										{@const flowsRunResourcePath = resourcePaths.get('flows:run') || '*'}

										<div class="p-3 border rounded-lg bg-surface-secondary h-auto col-span-full">
											<div class="mb-3">
												<h4 class="text-sm font-medium text-primary mb-2">Run Jobs</h4>
												<p class="text-xs text-tertiary">Enable running scripts and flows</p>
											</div>

											<div class="space-y-4">
												<!-- Scripts Run -->
												<div>
													<div class="flex items-center gap-2 mb-2">
														<input
															type="checkbox"
															id="scripts-run"
															checked={scriptsRunSelected}
															{disabled}
															onchange={(e) =>
																handleScopeWithResourcePath(
																	'scripts:run',
																	e.currentTarget.checked,
																	scriptsRunResourcePath
																)}
															class="w-4 h-4 text-blue-600 bg-surface border-gray-300 rounded focus:ring-blue-500 focus:ring-2"
														/>
														<label
															for="scripts-run"
															class="text-sm font-medium text-primary cursor-pointer flex-1"
														>
															Run Scripts
														</label>
														{#if scriptsRunSelected}
															<Popover closeOnOtherPopoverOpen contentClasses="p-3">
																{#snippet trigger()}
																	<Button size="xs" variant="border">
																		<Settings size={14} />
																	</Button>
																{/snippet}
																{#snippet content({ close })}
																	<div class="w-72">
																		<label class="block text-xs font-medium text-secondary mb-2">
																			Scripts Resource Path
																		</label>
																		<input
																			type="text"
																			value={scriptsRunResourcePath}
																			placeholder="e.g., f/folder/*, script_name, *"
																			{disabled}
																			oninput={(e) => {
																				handleScopeResourcePathChange(
																					'scripts:run',
																					e.currentTarget.value
																				)
																			}}
																			class="w-full text-sm px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-surface"
																		/>
																		<p class="text-xs text-tertiary mt-2">
																			Use '*' for all scripts, 'f/folder/*' for folder contents, or
																			specific paths.
																		</p>
																		<div class="flex gap-2 mt-3">
																			<Button
																				onclick={() => {
																					handleScopeResourcePathChange('scripts:run', '*')
																					close()
																				}}
																				size="xs"
																				variant="border">All (*)</Button
																			>
																			<Button onclick={close} size="xs">Done</Button>
																		</div>
																	</div>
																{/snippet}
															</Popover>
														{/if}
													</div>
													<p class="text-xs text-tertiary ml-6">Execute automation scripts</p>
												</div>

												<!-- Flows Run -->
												<div>
													<div class="flex items-center gap-2 mb-2">
														<input
															type="checkbox"
															id="flows-run"
															checked={flowsRunSelected}
															{disabled}
															onchange={(e) =>
																handleScopeWithResourcePath(
																	'flows:run',
																	e.currentTarget.checked,
																	flowsRunResourcePath
																)}
															class="w-4 h-4 text-blue-600 bg-surface border-gray-300 rounded focus:ring-blue-500 focus:ring-2"
														/>
														<label
															for="flows-run"
															class="text-sm font-medium text-primary cursor-pointer flex-1"
														>
															Run Flows
														</label>
														{#if flowsRunSelected}
															<Popover closeOnOtherPopoverOpen contentClasses="p-3">
																{#snippet trigger()}
																	<Button size="xs" variant="border">
																		<Settings size={14} />
																		{flowsRunResourcePath}
																	</Button>
																{/snippet}
																{#snippet content({ close })}
																	<div class="w-72">
																		<label class="block text-xs font-medium text-secondary mb-2">
																			Flows Resource Path
																		</label>
																		<input
																			type="text"
																			value={flowsRunResourcePath}
																			placeholder="e.g., f/folder/*, flow_name, *"
																			{disabled}
																			oninput={(e) => {
																				handleScopeResourcePathChange(
																					'flows:run',
																					e.currentTarget.value
																				)
																			}}
																			class="w-full text-sm px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-surface"
																		/>
																		<p class="text-xs text-tertiary mt-2">
																			Use '*' for all flows, 'f/folder/*' for folder contents, or
																			specific paths.
																		</p>
																		<div class="flex gap-2 mt-3">
																			<Button
																				onclick={() => {
																					handleScopeResourcePathChange('flows:run', '*')
																					close()
																				}}
																				size="xs"
																				variant="border">All (*)</Button
																			>
																			<Button onclick={close} size="xs">Done</Button>
																		</div>
																	</div>
																{/snippet}
															</Popover>
														{/if}
													</div>
													<p class="text-xs text-tertiary ml-6">Execute automation flows</p>
												</div>
											</div>
										</div>
									{:else}
										<div class="p-3 border rounded-lg bg-surface-secondary h-auto">
											<div class="flex items-start gap-2 mb-2">
												<div class="flex-shrink-0 mt-0.5">
													<input
														type="checkbox"
														id={`scope-${scope.value}`}
														checked={isSelected}
														{disabled}
														onchange={(e) =>
															handleIndividualScopeChange(
																scope,
																e.currentTarget.checked,
																scope.requires_resource_path ? resourcePath || '*' : undefined
															)}
														class="w-4 h-4 text-blue-600 bg-surface border-gray-300 rounded focus:ring-blue-500 focus:ring-2"
													/>
												</div>

												<div class="flex">
													<p class="font-medium text-secondary mb-2">
														{scope.label}
													</p>
													{#if scope.requires_resource_path && isSelected}
														<div class="flex justify-end">
															<Popover closeOnOtherPopoverOpen contentClasses="p-3">
																{#snippet trigger()}
																	<Button size="xs" variant="border">
																		<Settings size={14} />
																		Resource
																	</Button>
																{/snippet}
																{#snippet content({ close })}
																	<div class="w-72">
																		<p class="block text-xs font-medium text-secondary mb-2">
																			{scope.label} Resource Path
																		</p>
																		<input
																			type="text"
																			value={resourcePath}
																			placeholder="e.g., f/folder/*, resource_name, *"
																			{disabled}
																			oninput={(e) => {
																				handleScopeResourcePathChange(
																					scope.value,
																					e.currentTarget.value
																				)
																			}}
																			class="w-full text-sm px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-surface"
																		/>
																		<p class="text-xs text-tertiary mt-2">
																			Use '*' for all resources, 'f/folder/*' for folder contents,
																			or specific paths.
																		</p>
																		<div class="flex gap-2 mt-3">
																			<Button
																				onclick={() => {
																					handleScopeResourcePathChange(scope.value, '*')
																					close()
																				}}
																				size="xs"
																				variant="border">All (*)</Button
																			>
																			<Button onclick={close} size="xs">Done</Button>
																		</div>
																	</div>
																{/snippet}
															</Popover>
														</div>
													{/if}
												</div>
											</div>
										</div>
									{/if}
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
