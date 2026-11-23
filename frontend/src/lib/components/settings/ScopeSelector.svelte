<script lang="ts">
	import { type ScopeDomain, type ScopeDefinition, TokenService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { ChevronRight, Loader2, X } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import Checkbox from '../common/checkbox/Checkbox.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import PathPicker from '../pathPicker/PathPicker.svelte'
	import type { WorkspaceItemKind } from '../workspacePicker'
	import { twMerge } from 'tailwind-merge'

	import type { Snippet } from 'svelte'

	interface Props {
		selectedScopes?: string[]
		disabled?: boolean
		class?: string
		/** Renders above the scope card (header, selected chips and group list). */
		topSlot?: Snippet
		/** What an empty selection means, shown when no scope is selected. */
		emptyLabel?: string
		/** Workspace whose folders and items the path pickers list: the token's own, which an
		 *  edited token may hold apart from the workspace on screen. Defaults to the latter. */
		workspaceId?: string
	}

	interface ScopeState {
		isSelected: boolean
		resourcePaths: string[]
	}

	interface DomainState {
		isExpanded: boolean
		hasFullAccess: boolean
		scopes: Record<string, ScopeState>
	}

	interface ComponentState {
		domains: Record<string, DomainState>
	}

	let {
		selectedScopes = $bindable([]),
		disabled = false,
		class: className = '',
		topSlot,
		emptyLabel = 'No scopes selected. Token will have full access.',
		workspaceId
	}: Props = $props()

	let scopeDomains = $state<ScopeDomain[] | null>(null)
	let loading = $state(false)
	let error = $state<string | null>(null)

	let componentState = $state<ComponentState>({ domains: {} })

	function createScopeState(scope: ScopeDefinition): ScopeState {
		return {
			isSelected: false,
			resourcePaths: []
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

	function isScopeDisabled(scope: ScopeDefinition, domain: ScopeDomain): boolean {
		const domainState = getDomainState(domain.name)
		if (!domainState) return false

		if (scope.value.endsWith(':read')) {
			const writeScope = scope.value.replace(':read', ':write')
			const writeScopeState = domainState.scopes[writeScope]
			if (writeScopeState?.isSelected) {
				return true
			}
		}

		return false
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

			domainState.hasFullAccess = isFullAccessSelected(domain)

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

	// What the domain checkbox grants: the write scope (which implies read) and the run scopes.
	// A read-only domain (docs, data metrics, trigger history) has no write scope, so its checkbox
	// grants every scope it lists.
	function getFullAccessScopes(domain: ScopeDomain): string[] {
		const writeScope = domain.scopes.find((scope) => scope.value.endsWith(':write'))
		if (!writeScope) {
			return domain.scopes.map((scope) => scope.value)
		}
		const runScopes = domain.scopes.filter((scope) => /:run(:|$)/.test(scope.value))
		return [writeScope.value, ...runScopes.map((scope) => scope.value)]
	}

	function isFullAccessSelected(domain: ScopeDomain): boolean {
		const fullAccessScopes = getFullAccessScopes(domain)
		return (
			fullAccessScopes.length > 0 &&
			fullAccessScopes.every((value) =>
				selectedScopes.some((scope) => scope === value || scope.startsWith(value + ':'))
			)
		)
	}

	function toggleDomainExpansion(domainName: string) {
		// A search forces matching groups open; toggling then would flip hidden state.
		if (query !== '') return
		const domainState = getDomainState(domainName)
		if (domainState) {
			domainState.isExpanded = !domainState.isExpanded
		}
	}

	function handleDomainCheckboxChange(domain: ScopeDomain, checked: boolean) {
		const domainState = getDomainState(domain.name)
		if (!domainState) return

		domainState.hasFullAccess = checked

		if (checked) {
			selectedScopes = selectedScopes.filter(
				(scope) =>
					!domain.scopes.some(
						(domainScope) =>
							scope === domainScope.value || scope.startsWith(domainScope.value + ':')
					)
			)

			// A path-restricted scope already counts as selected for this checkbox (see
			// initializeDomainStates), so ticking it has to carry those paths over: re-adding the bare
			// scope would silently widen the grant from those paths to the whole domain.
			for (const scopeValue of getFullAccessScopes(domain)) {
				const scopeState = domainState.scopes[scopeValue]
				const paths = scopeState?.resourcePaths ?? []
				selectedScopes = [
					...selectedScopes,
					paths.length > 0 ? `${scopeValue}:${paths.join(',')}` : scopeValue
				]
				if (scopeState) {
					scopeState.isSelected = true
				}
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
			if (!checked) {
				scopeState.resourcePaths = []
				updateSelectedScopesForResourcePaths(scope.value, [])
			} else {
				const currentPaths = scopeState.resourcePaths || []
				updateSelectedScopesForResourcePaths(scope.value, currentPaths, false)
			}
		} else {
			selectedScopes = selectedScopes.filter(
				(s) => !s.startsWith(scope.value + ':') && s !== scope.value
			)
			if (checked) {
				selectedScopes = [...selectedScopes, scope.value]
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

		domainState.hasFullAccess = getFullAccessScopes(domain).every(
			(value) => domainState.scopes[value]?.isSelected
		)
	}

	function removeSelectedScope(scopeToRemove: string) {
		selectedScopes = selectedScopes.filter((scope) => scope !== scopeToRemove)

		const baseScopeValue =
			scopeToRemove.includes(':') && scopeToRemove.split(':').length > 2
				? scopeToRemove.split(':').slice(0, 2).join(':')
				: scopeToRemove

		const scopeState = getScopeState(baseScopeValue)
		if (scopeState) {
			if (scopeToRemove.includes(':') && scopeToRemove.split(':').length > 2) {
				const pathPart = scopeToRemove.substring(baseScopeValue.length + 1)
				const pathsToRemove = pathPart.split(',').map((p) => p.trim())
				scopeState.resourcePaths = scopeState.resourcePaths.filter(
					(path) => !pathsToRemove.includes(path)
				)

				if (scopeState.resourcePaths.length === 0) {
					scopeState.isSelected = false
				}
			} else {
				scopeState.isSelected = false
				scopeState.resourcePaths = []
			}

			updateDomainCheckboxState({ value: baseScopeValue } as ScopeDefinition)
		}
	}

	function clearAllScopes() {
		selectedScopes = []
		for (const domainState of Object.values(componentState.domains)) {
			domainState.hasFullAccess = false
			domainState.isExpanded = false
			for (const scopeState of Object.values(domainState.scopes)) {
				scopeState.isSelected = false
				scopeState.resourcePaths = []
			}
		}
	}

	const hasAdministratorScope = $derived(selectedScopes.includes('*'))

	let search = $state('')
	const query = $derived(search.trim().toLowerCase())

	function matchesQuery(text: string | null | undefined): boolean {
		return !!text && text.toLowerCase().includes(query)
	}

	// A domain matching by name or description shows all its scopes; otherwise only the matching ones.
	function getVisibleScopes(domain: ScopeDomain): ScopeDefinition[] {
		if (query === '' || matchesQuery(domain.name) || matchesQuery(domain.description)) {
			return domain.scopes
		}
		return domain.scopes.filter((scope) => matchesQuery(scope.label) || matchesQuery(scope.value))
	}

	const visibleDomains = $derived(
		(scopeDomains ?? []).filter((domain) => getVisibleScopes(domain).length > 0)
	)

	$effect(() => {
		if (scopeDomains && componentState.domains) {
			syncSelectedScopesWithState()
		}
	})

	/** Item kinds whose paths a scope restricts, for the path picker. Scopes on other
	 * resources (variables, schedules, triggers, …) are offered folders and typed paths. */
	function pathPickerKinds(scopeValue: string): WorkspaceItemKind[] {
		if (scopeValue === 'jobs:run:scripts') return ['script']
		if (scopeValue === 'jobs:run:flows') return ['flow']
		switch (scopeValue.split(':')[0]) {
			case 'jobs':
				return ['script', 'flow']
			case 'scripts':
				return ['script']
			case 'flows':
				return ['flow']
			case 'apps':
			case 'raw_apps':
				return ['app']
			default:
				return []
		}
	}

	function validateResourcePath(path: string): string | null {
		if (!path.trim()) return 'Path cannot be empty'

		const trimmedPath = path.trim()

		if (trimmedPath === '*') return null

		if (trimmedPath === 'u/*' || trimmedPath === 'f/*') return null

		if (!trimmedPath.startsWith('u/') && !trimmedPath.startsWith('f/')) {
			return 'Path must start with u/ or f/'
		}

		const parts = trimmedPath.split('/')
		if (parts.length < 3) {
			return 'Expected path format: u/{user}/{resource} or f/{folder}/{resource}'
		}

		if (parts[1] === '') {
			return 'Username/folder name cannot be empty'
		}
		const last = parts[parts.length - 1]
		if (last.length === 0) {
			return 'Resource name cannot be empty'
		}

		if (last === '*') return null

		if (last.includes('*')) {
			return 'Wildcards can only be used as the full resource name (*)'
		}

		return null
	}

	function setResourcePaths(scopeValue: string, paths: string[]) {
		const scopeState = getScopeState(scopeValue)
		if (!scopeState) return
		scopeState.resourcePaths = paths
		// No path left keeps the scope, unrestricted.
		updateSelectedScopesForResourcePaths(scopeValue, paths, false)
	}

	function updateSelectedScopesForResourcePaths(
		scopeValue: string,
		paths: string[],
		removeScope = true
	) {
		selectedScopes = selectedScopes.filter(
			(s) => !s.startsWith(scopeValue + ':') && s !== scopeValue
		)

		const scopeState = getScopeState(scopeValue)
		if (!scopeState) return

		if (paths.length > 0 || !removeScope) {
			selectedScopes = [
				...selectedScopes,
				paths.length > 0 ? `${scopeValue}:${paths.join(',')}` : scopeValue
			]
			scopeState.isSelected = true
		} else {
			scopeState.isSelected = false
		}

		updateDomainCheckboxState({ value: scopeValue } as ScopeDefinition)
	}

	function syncSelectedScopesWithState() {
		if (!scopeDomains) return

		for (const domain of scopeDomains) {
			const domainState = getDomainState(domain.name)
			if (!domainState) continue

			domainState.hasFullAccess = isFullAccessSelected(domain)

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

<!-- The label can be a long comma-joined path list. It must never widen its container, which would
     push the per-scope path buttons out of the panel, so it wraps: an ellipsis would hide the
     paths being granted. -->
{#snippet scopeChip(
	label: string,
	removeTitle: string,
	onRemove: (e: MouseEvent) => void,
	removeDisabled: boolean
)}
	<span
		class="inline-flex items-center gap-1 min-w-0 max-w-full pl-1.5 pr-1 py-0.5 rounded bg-surface-secondary border border-border-light font-mono text-2xs text-emphasis"
	>
		<span class="break-all">{label}</span>
		<button
			type="button"
			onclick={onRemove}
			class="flex-shrink-0 rounded text-secondary hover:text-emphasis hover:bg-surface-hover disabled:opacity-50"
			title={removeTitle}
			aria-label={removeTitle}
			disabled={removeDisabled}
		>
			<X size={12} />
		</button>
	</span>
{/snippet}

<div class="w-full flex flex-col gap-2 {className}">
	{#if loading}
		<div class="flex items-center gap-2 py-2 text-xs text-secondary">
			<Loader2 size={16} class="animate-spin" />
			Loading scopes
		</div>
	{:else if error}
		<div class="flex items-center gap-2">
			<p class="text-xs text-red-500">{error}</p>
			<Button onclick={fetchScopeDomains} variant="default" unifiedSize="sm">Try again</Button>
		</div>
	{:else if scopeDomains}
		{#if topSlot}
			{@render topSlot()}
		{/if}

		<div class="rounded-md border border-border-light bg-surface-tertiary overflow-hidden">
			<div class="flex items-center justify-between gap-2 h-9 px-3 border-b border-border-light">
				<span class="text-xs font-semibold text-emphasis whitespace-nowrap">
					Scopes
					<span class="font-normal text-secondary">· {selectedScopes.length} selected</span>
				</span>
				<!-- Search stays rightmost so Clear appearing does not move it. -->
				<div class="flex items-center gap-2 min-w-0">
					{#if selectedScopes.length > 0}
						<Button onclick={clearAllScopes} {disabled} unifiedSize="sm" variant="default">
							Clear selected scopes
						</Button>
					{/if}
					<TextInput
						bind:value={search}
						size="sm"
						class="w-56 max-w-full"
						inputProps={{ placeholder: 'Search scopes', 'aria-label': 'Search scopes' }}
					/>
				</div>
			</div>

			<!-- The empty text and one row of chips share a min height, so the first selection
			     does not push down the list being clicked. -->
			<div class="px-3 py-2 border-b border-border-light bg-surface-secondary">
				{#if selectedScopes.length === 0 || hasAdministratorScope}
					<p class="flex items-center min-h-6 text-2xs text-secondary">
						{selectedScopes.length === 0
							? emptyLabel
							: 'Administrator scope grants full access to all resources.'}
					</p>
				{:else}
					<div class="flex flex-wrap items-center gap-1 min-h-6 max-h-24 overflow-y-auto">
						{#each selectedScopes as scope}
							{@render scopeChip(scope, 'Remove scope', () => removeSelectedScope(scope), disabled)}
						{/each}
					</div>
				{/if}
			</div>

			<div class="max-h-96 overflow-y-auto divide-y divide-border-light">
				{#each visibleDomains as domain (domain.name)}
					{@const domainState = getDomainState(domain.name)}
					{@const isExpanded = query !== '' || domainState?.isExpanded || false}
					{@const isDomainSelected = domainState?.hasFullAccess || false}
					<!-- Counts a read scope implied by write, which its row shows as checked. -->
					{@const selectedCount = domain.scopes.filter(
						(s) => domainState?.scopes[s.value]?.isSelected || isScopeDisabled(s, domain)
					).length}

					<div>
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-surface-hover transition-colors"
							onclick={() => toggleDomainExpansion(domain.name)}
						>
							<ChevronRight
								size={16}
								class="flex-shrink-0 text-secondary transition-transform duration-200 {isExpanded
									? 'rotate-90'
									: ''}"
							/>
							<Checkbox
								checked={isDomainSelected}
								indeterminate={!isDomainSelected && selectedCount > 0}
								{disabled}
								title={`Grant every ${domain.name} scope`}
								onChange={() => handleDomainCheckboxChange(domain, !isDomainSelected)}
								onClick={(e) => e.stopPropagation()}
							/>
							<div class="flex-1 min-w-0">
								<div class="text-xs font-semibold text-emphasis">{domain.name}</div>
								{#if domain.description}
									<div class="text-2xs text-secondary">{domain.description}</div>
								{/if}
							</div>
							{#if selectedCount > 0}
								<span class="flex-shrink-0 text-2xs text-secondary">
									{selectedCount} of {domain.scopes.length}
								</span>
							{/if}
						</div>

						{#if isExpanded}
							<!-- One grid for the group's rows, so path pickers share a left edge and take
							     the width the labels leave. -->
							<div
								class="grid grid-cols-[auto_minmax(12rem,1fr)] items-center gap-x-4 gap-y-1 pb-2 pl-[3.25rem] pr-3"
							>
								{#each getVisibleScopes(domain) as scope (scope.value)}
									{@const isSelected = domainState?.scopes[scope.value]?.isSelected || false}
									{@const isImplied = !isSelected && isScopeDisabled(scope, domain)}
									{@const isDisabled = disabled || isScopeDisabled(scope, domain)}

									<div class="contents">
										<label
											class={twMerge(
												'flex items-center gap-2 min-w-0 min-h-7',
												isDisabled ? 'cursor-not-allowed' : 'cursor-pointer'
											)}
											title={isImplied ? 'Granted by the write scope, on the same paths' : undefined}
										>
											<!-- A read scope covered by the selected write scope shows as granted. -->
											<Checkbox
												checked={isSelected || isImplied}
												disabled={isDisabled}
												onChange={() => handleIndividualScopeChange(scope, !isSelected)}
											/>
											<span
												class={twMerge(
													'text-xs truncate',
													isDisabled ? 'text-disabled' : 'text-primary'
												)}
											>
												{scope.label}
											</span>
											<span class="font-mono text-2xs text-hint truncate">{scope.value}</span>
										</label>
										{#if scope.requires_resource_path && isSelected}
											<PathPicker
												bind:value={
													() => domainState?.scopes[scope.value]?.resourcePaths ?? [],
													(paths) => setResourcePaths(scope.value, paths)
												}
												kinds={pathPickerKinds(scope.value)}
												validate={validateResourcePath}
												{workspaceId}
												{disabled}
											/>
										{:else}
											<div></div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{:else}
					<p class="px-3 py-4 text-xs text-secondary">No scope matches "{search.trim()}".</p>
				{/each}
			</div>
		</div>
	{/if}
</div>
