<script lang="ts">
	import Select from '$lib/components/select/Select.svelte'
	import AutocompleteSelect from '$lib/components/select/AutocompleteSelect.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ChevronDown, Moon, Sun, CircleDot } from 'lucide-svelte'

	let darkMode: boolean = $state(false)

	function toggleTheme() {
		if (!document.documentElement.classList.contains('dark')) {
			document.documentElement.classList.add('dark')
			window.localStorage.setItem('dark-mode', 'dark')
		} else {
			document.documentElement.classList.remove('dark')
			window.localStorage.setItem('dark-mode', 'light')
		}
	}

	// -- Data --
	const fruits = [
		{ label: 'Apple', value: 'apple' },
		{ label: 'Banana', value: 'banana' },
		{ label: 'Cherry', value: 'cherry' },
		{ label: 'Date', value: 'date' },
		{ label: 'Elderberry', value: 'elderberry' },
		{ label: 'Fig', value: 'fig' },
		{ label: 'Grape', value: 'grape' }
	]

	const emails = [
		{ label: 'alice@example.com', value: 'alice@example.com' },
		{ label: 'bob@example.com', value: 'bob@example.com' },
		{ label: 'charlie@example.com', value: 'charlie@example.com' }
	]

	const schemas = [
		{ label: 'public', value: 'public' },
		{ label: 'analytics', value: 'analytics' },
		{ label: 'staging', value: 'staging' }
	]

	const groupedItems = [
		{ label: 'JavaScript', value: 'js', group: 'Frontend' },
		{ label: 'TypeScript', value: 'ts', group: 'Frontend' },
		{ label: 'Svelte', value: 'svelte', group: 'Frontend' },
		{ label: 'Python', value: 'python', group: 'Backend' },
		{ label: 'Rust', value: 'rust', group: 'Backend' },
		{ label: 'Go', value: 'go', group: 'Backend' },
		{ label: 'PostgreSQL', value: 'postgres', group: 'Database' },
		{ label: 'MySQL', value: 'mysql', group: 'Database' }
	]

	const longList = Array.from({ length: 50 }, (_, i) => ({
		label: `Item ${i + 1} - ${['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon'][i % 5]}`,
		value: `item-${i + 1}`
	}))

	const disabledItems = [
		{ label: 'Available', value: 'available' },
		{ label: 'Disabled option', value: 'disabled', disabled: true },
		{ label: 'Another available', value: 'another' },
		{ label: 'Also disabled', value: 'also-disabled', disabled: true }
	]

	// -- State for each test case --
	let basic: string | undefined = $state(undefined)
	let preselected: string | undefined = $state('banana')
	let clearableVal: string | undefined = $state('cherry')
	let clearableWithCallback: string | undefined = $state('apple')
	let errorVal: string | undefined = $state(undefined)
	let disabledVal: string | undefined = $state('apple')
	let loadingVal: string | undefined = $state(undefined)
	let loadingWithVal: string | undefined = $state('banana')
	let createVal: string | undefined = $state(undefined)
	let allowUserInputVal: string | undefined = $state(undefined)
	let allowUserInputPreselected: string | undefined = $state('alice@example.com')
	let transformVal: string | undefined = $state('public')
	let rightIconVal: string | undefined = $state(undefined)
	let groupedVal: string | undefined = $state(undefined)
	let longListVal: string | undefined = $state(undefined)
	let disabledItemsVal: string | undefined = $state(undefined)
	let snippetVal: string | undefined = $state(undefined)
	let noItemsVal: string | undefined = $state(undefined)
	let smallVal: string | undefined = $state(undefined)
	let largeVal: string | undefined = $state(undefined)

	// Loading simulation
	let fakeLoading = $state(true)
	let fakeLoadingItems: typeof fruits | undefined = $state(undefined)

	function simulateLoad() {
		fakeLoading = true
		fakeLoadingItems = undefined
		setTimeout(() => {
			fakeLoadingItems = fruits
			fakeLoading = false
		}, 2000)
	}
	simulateLoad()

	// Logging
	let log: string[] = $state([])
	function addLog(msg: string) {
		log = [`${new Date().toLocaleTimeString()} - ${msg}`, ...log].slice(0, 20)
	}
</script>

<div class="p-8 flex flex-col gap-8 max-w-5xl mx-auto bg-surface-secondary min-h-screen pb-32">
	<header class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<h1 class="text-2xl font-semibold text-emphasis">Select Component Sandbox</h1>
			<Button
				variant="subtle"
				size="xs"
				startIcon={{ icon: darkMode ? Sun : Moon }}
				onclick={toggleTheme}
			>
				{darkMode ? 'Light mode' : 'Dark mode'}
			</Button>
		</div>
		<p class="text-xs text-secondary">
			Comprehensive test page for all Select component usage patterns.
		</p>
	</header>

	<!-- BASIC SELECTION -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Basic Selection
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Default</h4>
				<p class="text-2xs text-secondary">Basic select with placeholder</p>
				<Select items={fruits} bind:value={basic} placeholder="Pick a fruit" />
				<p class="text-2xs text-primary">Value: {JSON.stringify(basic)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Pre-selected</h4>
				<p class="text-2xs text-secondary">Starts with a value, open shows it pre-filled</p>
				<Select items={fruits} bind:value={preselected} placeholder="Pick a fruit" />
				<p class="text-2xs text-primary">Value: {JSON.stringify(preselected)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">No items</h4>
				<p class="text-2xs text-secondary">Empty items array, custom noItemsMsg</p>
				<Select
					items={[] as { label: string; value: string }[]}
					bind:value={noItemsVal}
					placeholder="Nothing here"
					noItemsMsg="No options available"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(noItemsVal)}</p>
			</div>
		</div>
	</section>

	<!-- CLEARABLE & ERROR -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Clearable & Validation
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Clearable</h4>
				<p class="text-2xs text-secondary">X button to clear value</p>
				<Select items={fruits} bind:value={clearableVal} clearable placeholder="Pick a fruit" />
				<p class="text-2xs text-primary">Value: {JSON.stringify(clearableVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Clearable + onClear callback</h4>
				<p class="text-2xs text-secondary">Custom clear logic with logging</p>
				<Select
					items={fruits}
					bind:value={clearableWithCallback}
					clearable
					onClear={() => {
						clearableWithCallback = undefined
						addLog('onClear fired')
					}}
					placeholder="Pick a fruit"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(clearableWithCallback)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Error state</h4>
				<p class="text-2xs text-secondary">Red border when error=true</p>
				<Select
					items={fruits}
					bind:value={errorVal}
					error={!errorVal}
					placeholder="Required field"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(errorVal)}</p>
				{#if !errorVal}
					<p class="text-2xs text-red-500">This field is required</p>
				{/if}
			</div>
		</div>
	</section>

	<!-- DISABLED & LOADING -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Disabled & Loading
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Disabled</h4>
				<p class="text-2xs text-secondary">Cannot interact</p>
				<Select
					items={fruits}
					bind:value={disabledVal}
					disabled={true}
					placeholder="Disabled select"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(disabledVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Loading (no value)</h4>
				<p class="text-2xs text-secondary">Shows spinner + "Loading..." placeholder</p>
				<Select
					items={fakeLoadingItems}
					bind:value={loadingVal}
					loading={fakeLoading}
					placeholder="Pick a fruit"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(loadingVal)}</p>
				<Button variant="subtle" size="xs" onclick={simulateLoad}>Reload (2s)</Button>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Loading (with value)</h4>
				<p class="text-2xs text-secondary">Loading spinner but input is not disabled</p>
				<Select
					items={fakeLoadingItems}
					bind:value={loadingWithVal}
					loading={fakeLoading}
					placeholder="Pick a fruit"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(loadingWithVal)}</p>
			</div>
		</div>
	</section>

	<!-- USER INPUT & CREATE -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			User Input & Item Creation
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">createText + onCreateItem</h4>
				<p class="text-2xs text-secondary">Manual "Use custom:" button in dropdown</p>
				<Select
					items={schemas}
					bind:value={createVal}
					createText="Create schema:"
					onCreateItem={(v) => {
						createVal = v
						addLog(`onCreateItem: ${v}`)
					}}
					placeholder="Search or create schema..."
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(createVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">AutocompleteSelect</h4>
				<p class="text-2xs text-secondary">Type anything, value syncs live. Dropdown hides when no matches.</p>
				<AutocompleteSelect
					items={emails}
					bind:value={allowUserInputVal}
					placeholder="Select or type an email"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(allowUserInputVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">AutocompleteSelect (pre-selected)</h4>
				<p class="text-2xs text-secondary">Pre-filled value, open shows it, can edit</p>
				<AutocompleteSelect
					items={emails}
					bind:value={allowUserInputPreselected}
					clearable
					placeholder="Select or type an email"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(allowUserInputPreselected)}</p>
			</div>
		</div>
	</section>

	<!-- TRANSFORM & ICONS -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Transform & Icons
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">transformInputSelectedText</h4>
				<p class="text-2xs text-secondary">Shows "Schema: X" when closed</p>
				<Select
					items={schemas}
					bind:value={transformVal}
					transformInputSelectedText={(s) => `Schema: ${s}`}
					placeholder="Search schema..."
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(transformVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">RightIcon</h4>
				<p class="text-2xs text-secondary">Custom icon on the right</p>
				<Select
					items={schemas}
					bind:value={rightIconVal}
					RightIcon={ChevronDown}
					placeholder="Pick a schema"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(rightIconVal)}</p>
			</div>
		</div>
	</section>

	<!-- GROUPED & LONG LISTS -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Groups, Long Lists & Disabled Items
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">groupBy</h4>
				<p class="text-2xs text-secondary">Items grouped by category</p>
				<Select
					items={groupedItems}
					bind:value={groupedVal}
					groupBy={(item) => item.group}
					placeholder="Pick a language"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(groupedVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Long list (50 items)</h4>
				<p class="text-2xs text-secondary">Scrollable dropdown, search to filter</p>
				<Select items={longList} bind:value={longListVal} placeholder="Search 50 items..." />
				<p class="text-2xs text-primary">Value: {JSON.stringify(longListVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Disabled items</h4>
				<p class="text-2xs text-secondary">Some items cannot be selected</p>
				<Select
					items={disabledItems}
					bind:value={disabledItemsVal}
					placeholder="Pick an option"
				/>
				<p class="text-2xs text-primary">Value: {JSON.stringify(disabledItemsVal)}</p>
			</div>
		</div>
	</section>

	<!-- SIZES -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">Sizes</h2>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Small (sm)</h4>
				<Select items={fruits} bind:value={smallVal} size="sm" placeholder="Small" />
				<p class="text-2xs text-primary">Value: {JSON.stringify(smallVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Medium (md) - default</h4>
				<Select items={fruits} bind:value={basic} placeholder="Medium (default)" />
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">Large (lg)</h4>
				<Select items={fruits} bind:value={largeVal} size="lg" placeholder="Large" />
				<p class="text-2xs text-primary">Value: {JSON.stringify(largeVal)}</p>
			</div>
		</div>
	</section>

	<!-- SNIPPETS -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Snippets
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">startSnippet + endSnippet</h4>
				<p class="text-2xs text-secondary">Custom content before/after each item label</p>
				<Select items={fruits} bind:value={snippetVal} placeholder="Pick a fruit">
					{#snippet startSnippet({ item })}
						<CircleDot
							size={12}
							class={item.value === snippetVal ? 'text-green-500' : 'text-gray-300'}
						/>
					{/snippet}
					{#snippet endSnippet({ item })}
						<span class="text-2xs text-secondary ml-auto">{item.value}</span>
					{/snippet}
				</Select>
				<p class="text-2xs text-primary">Value: {JSON.stringify(snippetVal)}</p>
			</div>

			<div class="flex flex-col gap-3 p-5 border border-border-light rounded-lg bg-surface">
				<h4 class="text-xs font-semibold text-emphasis">bottomSnippet</h4>
				<p class="text-2xs text-secondary">Custom content at the bottom of dropdown</p>
				<Select items={fruits} bind:value={snippetVal} placeholder="Pick a fruit">
					{#snippet bottomSnippet({ close })}
						<div class="p-2 border-t border-border-light">
							<button
								class="text-2xs text-accent hover:underline"
								onclick={() => {
									addLog('Bottom action clicked')
									close()
								}}
							>
								+ Add custom option
							</button>
						</div>
					{/snippet}
				</Select>
			</div>
		</div>
	</section>

	<!-- EVENT LOG -->
	<section class="flex flex-col gap-4">
		<h2 class="text-lg font-semibold text-emphasis border-b border-border-light pb-2">
			Event Log
		</h2>
		<div class="p-4 bg-surface border border-border-light rounded-lg max-h-48 overflow-y-auto">
			{#if log.length === 0}
				<p class="text-2xs text-secondary">Interact with components above to see events...</p>
			{:else}
				{#each log as entry}
					<p class="text-2xs text-primary font-mono">{entry}</p>
				{/each}
			{/if}
		</div>
	</section>
</div>

<DarkModeObserver bind:darkMode />
