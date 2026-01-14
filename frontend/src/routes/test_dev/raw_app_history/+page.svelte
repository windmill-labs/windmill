<script lang="ts">
	import RawAppHistoryList from '$lib/components/raw_apps/RawAppHistoryList.svelte'
	import type {
		HistoryEntry,
		HistoryBranch
	} from '$lib/components/raw_apps/RawAppHistoryManager.svelte.ts'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Moon, Sun, Plus, Trash, RotateCcw, GitBranch } from 'lucide-svelte'

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

	// Mock data generators
	let entryIdCounter = $state(0)
	let branchIdCounter = $state(0)

	function createMockEntry(hoursAgo: number = 0): HistoryEntry {
		const timestamp = new Date()
		timestamp.setHours(timestamp.getHours() - hoursAgo)
		return {
			id: entryIdCounter++,
			timestamp,
			files: { 'index.html': `<div>Entry ${entryIdCounter}</div>` },
			runnables: {},
			summary: `Snapshot ${entryIdCounter}`,
			data: {
				tables: [],
				datatable: undefined,
				schema: undefined
			}
		}
	}

	// Sample entries for testing
	let entries = $state<HistoryEntry[]>([
		createMockEntry(48), // 2 days ago
		createMockEntry(24), // 1 day ago
		createMockEntry(12), // 12 hours ago
		createMockEntry(6), // 6 hours ago
		createMockEntry(1), // 1 hour ago
		createMockEntry(0) // now
	])

	// Sample branches for testing
	let branches = $state<HistoryBranch[]>([])

	let selectedId = $state<number | undefined>(undefined)

	function handleSelect(id: number) {
		selectedId = id
		console.log('Selected entry:', id)
	}

	// Test actions
	function addEntry() {
		entries = [...entries, createMockEntry(0)]
	}

	function addBranch() {
		if (entries.length < 2) return

		// Fork from 3rd entry (or last available)
		const forkIndex = Math.min(2, entries.length - 1)
		const forkPointId = entries[forkIndex].id

		const branchEntries: HistoryEntry[] = [createMockEntry(0.5), createMockEntry(0.25)]

		branches = [
			...branches,
			{
				id: branchIdCounter++,
				forkPointId,
				entries: branchEntries
			}
		]
	}

	function clearAll() {
		entries = []
		branches = []
		selectedId = undefined
		entryIdCounter = 0
		branchIdCounter = 0
	}

	function resetToDefault() {
		clearAll()
		entries = [
			createMockEntry(48),
			createMockEntry(24),
			createMockEntry(12),
			createMockEntry(6),
			createMockEntry(1),
			createMockEntry(0)
		]
	}
</script>

<div class="p-8 flex flex-col gap-8 max-w-4xl mx-auto bg-surface-secondary min-h-screen pb-32">
	<header class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<h1 class="text-2xl font-semibold text-emphasis">RawAppHistoryList Test</h1>
			<Button
				variant="subtle"
				size="xs"
				startIcon={{ icon: darkMode ? Sun : Moon }}
				onclick={toggleTheme}
				title={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
			>
				{darkMode ? 'Light mode' : 'Dark mode'}
			</Button>
		</div>
		<p class="text-xs text-secondary max-w-3xl">
			Isolated test page for the RawAppHistoryList component. Test the timeline visualization with
			branches.
		</p>
	</header>

	<!-- Controls -->
	<div
		class="flex flex-col gap-4 p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
	>
		<h2 class="text-sm font-semibold text-emphasis">Test Controls</h2>
		<div class="flex flex-wrap gap-2">
			<Button variant="default" size="xs" startIcon={{ icon: Plus }} onclick={addEntry}>
				Add Entry
			</Button>
			<Button variant="default" size="xs" startIcon={{ icon: GitBranch }} onclick={addBranch}>
				Add Branch
			</Button>
			<Button variant="subtle" size="xs" startIcon={{ icon: RotateCcw }} onclick={resetToDefault}>
				Reset
			</Button>
			<Button variant="subtle" size="xs" startIcon={{ icon: Trash }} onclick={clearAll} destructive>
				Clear All
			</Button>
		</div>
		<div class="text-2xs text-tertiary">
			Entries: {entries.length} | Branches: {branches.length} | Selected: {selectedId ?? 'none'}
		</div>
	</div>

	<!-- Main test area -->
	<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
		<!-- History List Component -->
		<div
			class="flex flex-col gap-4 p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
		>
			<h2 class="text-sm font-semibold text-emphasis">History List</h2>
			<div class="max-h-[500px] overflow-y-auto">
				<RawAppHistoryList {entries} {branches} {selectedId} onSelect={handleSelect} />
			</div>
		</div>

		<!-- Selected Entry Details -->
		<div
			class="flex flex-col gap-4 p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
		>
			<h2 class="text-sm font-semibold text-emphasis">Selected Entry Details</h2>
			{#if selectedId !== undefined}
				{@const selectedEntry =
					entries.find((e) => e.id === selectedId) ||
					branches.flatMap((b) => b.entries).find((e) => e.id === selectedId)}
				{#if selectedEntry}
					<div class="space-y-2 text-xs">
						<div
							><span class="text-tertiary">ID:</span>
							<span class="text-primary">{selectedEntry.id}</span></div
						>
						<div
							><span class="text-tertiary">Timestamp:</span>
							<span class="text-primary">{selectedEntry.timestamp.toLocaleString()}</span></div
						>
						<div
							><span class="text-tertiary">Summary:</span>
							<span class="text-primary">{selectedEntry.summary}</span></div
						>
						<div class="text-tertiary">Files:</div>
						<pre class="bg-surface-tertiary p-2 rounded text-2xs overflow-x-auto"
							>{JSON.stringify(selectedEntry.files, null, 2)}</pre
						>
					</div>
				{/if}
			{:else}
				<p class="text-xs text-tertiary">Select an entry from the history list to see details.</p>
			{/if}
		</div>
	</div>

	<!-- State Inspector -->
	<div
		class="flex flex-col gap-4 p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
	>
		<h2 class="text-sm font-semibold text-emphasis">State Inspector</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
			<div>
				<h3 class="text-2xs font-medium text-tertiary mb-2">Entries ({entries.length})</h3>
				<pre class="bg-surface-tertiary p-2 rounded text-2xs max-h-48 overflow-auto"
					>{JSON.stringify(
						entries.map((e) => ({
							id: e.id,
							timestamp: e.timestamp.toISOString(),
							summary: e.summary
						})),
						null,
						2
					)}</pre
				>
			</div>
			<div>
				<h3 class="text-2xs font-medium text-tertiary mb-2">Branches ({branches.length})</h3>
				<pre class="bg-surface-tertiary p-2 rounded text-2xs max-h-48 overflow-auto"
					>{JSON.stringify(
						branches.map((b) => ({
							id: b.id,
							forkPointId: b.forkPointId,
							entries: b.entries.map((e) => e.id)
						})),
						null,
						2
					)}</pre
				>
			</div>
		</div>
	</div>
</div>

<DarkModeObserver bind:darkMode />
