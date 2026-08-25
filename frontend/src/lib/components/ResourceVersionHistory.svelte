<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ResourceService, type ResourceVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import Button from './common/button/Button.svelte'
	import HighlightCode from './HighlightCode.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Code, Diff, History, Loader2, RotateCcw, Trash2 } from 'lucide-svelte'
	import Alert from './common/alert/Alert.svelte'
	import VersionListItem from './VersionListItem.svelte'
	import { displayDate } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'

	let {
		path,
		workspace = undefined,
		canRestore = true,
		canClear = false,
		onRestore = undefined
	}: {
		path: string
		workspace?: string
		canRestore?: boolean
		canClear?: boolean
		onRestore?: () => void
	} = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	let versions = $state<ResourceVersion[] | undefined>(undefined)
	// Which row is highlighted, updated on click. `loaded` is dropped the moment the selection
	// moves and only reinstated once its own fetch lands, so the pane can never show one version's
	// JSON under another version's highlight.
	let selectedId = $state<number | undefined>(undefined)
	// `id` addresses the version, `version` is what it is called: the id is unique across every
	// resource, so it is no indication of how many times this one has been saved.
	let loaded = $state<
		{ id: number; version: number; value: string; missing: string[] } | undefined
	>(undefined)
	// Undefined until the newest version's value arrives, which is what "Diff with current" needs.
	// Fetched without blocking the list, so an absent baseline disables the diff rather than
	// holding up the drawer everyone else opened to read.
	let currentValue = $state<string | undefined>(undefined)
	// The backend decides this, rather than sending the resource type for the UI to test, so the
	// list of never-versioned types stays stated once (INTERNAL_RESOURCE_TYPES) instead of being
	// restated here in TypeScript as well as in the recording trigger's SQL.
	let neverVersioned = $state(false)
	// Bumped per load so a response for a path the drawer has since moved off cannot land. The
	// component is reused rather than remounted when `path` changes, and `loaded` is what Restore
	// acts on, so a late response could otherwise restore the previous resource's version.
	let loadGeneration = 0
	let view = $state<'value' | 'diff'>('value')
	let restoring = $state(false)
	// The newest version holds the live value — recording is a database trigger, so every write
	// mints one — which makes a diff of it against current necessarily empty.
	let isCurrent = $derived(loaded !== undefined && versions?.[0]?.id === loaded.id)
	let clearing = $state(false)
	let confirmingClear = $state(false)

	function pretty(value: unknown): string {
		return JSON.stringify(value ?? null, null, 2)
	}

	async function loadVersions() {
		const generation = ++loadGeneration
		// Cleared before the first await, not after: everything below is actionable state, and
		// leaving it pointing at the previous resource is what lets Restore act on the wrong one.
		versions = undefined
		loaded = undefined
		selectedId = undefined
		currentValue = undefined
		// Including the confirmation: it is armed for one resource and acts on whichever the
		// drawer currently points at, so leaving it up across a retarget would let Confirm delete
		// a history the user never asked to clear.
		confirmingClear = false
		const history = await ResourceService.getResourceHistory({
			workspace: effectiveWorkspace,
			path
		})
		if (generation !== loadGeneration) return
		neverVersioned = history.versioned === false
		// Nothing is selected on open, as in ScriptVersionHistory. Pre-selecting a version would
		// fill the pane with JSON that reads as the resource's value without being it, since the
		// drawer opens on the value view rather than the diff.
		versions = history.versions ?? []
		void loadDiffBaseline(versions[0]?.id, generation)
	}

	/// The diff baseline is the newest version, not the resource's live value, even though the two
	/// hold the same thing: versions are immutable and addressed by id, so this cannot disagree
	/// with the list it came from, where reading `resource` would reintroduce a mutable second
	/// source. Deliberately not awaited by the caller — only the diff view needs it, which is two
	/// interactions away, so the list paints as soon as it arrives.
	async function loadDiffBaseline(newestId: number | undefined, generation: number) {
		if (newestId === undefined) return
		try {
			const newest = await fetchVersion(newestId)
			if (generation === loadGeneration) {
				currentValue = newest.value
			}
		} catch {
			// Leaves `currentValue` undefined, which hides the diff rather than offering one
			// against nothing.
		}
	}

	async function fetchVersion(id: number) {
		const version = await ResourceService.getResourceVersion({
			workspace: effectiveWorkspace,
			id
		})
		return {
			id,
			version: version.version,
			value: pretty(version.value),
			missing: version.missing_references ?? []
		}
	}

	async function selectVersion(
		id: number | undefined,
		number: number | undefined,
		generation = loadGeneration
	) {
		selectedId = id
		// Dropped up front rather than left in place while the new value is in flight: keeping it
		// would highlight the clicked row while the pane still rendered the previous version, and
		// a failed fetch would leave that mismatch on screen indefinitely.
		loaded = undefined
		if (id === undefined) {
			return
		}
		try {
			const version = await fetchVersion(id)
			// Assigned as one object so the id keying the editor and the value it displays can
			// never be out of step, whatever order the requests come back in.
			if (selectedId === id && generation === loadGeneration) {
				loaded = version
			}
		} catch (err) {
			if (selectedId === id && generation === loadGeneration) {
				selectedId = undefined
				sendUserToast(`Could not load version ${number}`, true)
			}
		}
	}

	async function restore() {
		// loaded.id, never selectedId: restoring what the pane is showing. A selection whose value
		// has not arrived leaves `loaded` undefined, so this writes nothing rather than restoring a
		// version the user has not seen.
		const target = loaded
		if (target === undefined) return
		restoring = true
		try {
			await ResourceService.restoreResourceVersion({
				workspace: effectiveWorkspace,
				id: target.id
			})
			sendUserToast(`Restored ${path} to version ${target.version}`)
			onRestore?.()
			await loadVersions()
		} finally {
			restoring = false
		}
	}

	async function clearHistory() {
		clearing = true
		try {
			await ResourceService.clearResourceHistory({ workspace: effectiveWorkspace, path })
			sendUserToast(`Cleared past versions of ${path}`)
			confirmingClear = false
			await loadVersions()
		} finally {
			clearing = false
		}
	}

	$effect(() => {
		if (path && effectiveWorkspace) {
			loadVersions()
		}
	})
</script>

<Splitpanes class="h-full">
	<Pane size={20}>
		{#if versions === undefined}
			<div class="p-2 flex flex-col gap-2">
				{#each Array(4) as _}
					<Skeleton layout={[[2]]} />
				{/each}
			</div>
		{:else if versions.length === 0}
			<div class="p-4 text-tertiary text-xs">
				{#if neverVersioned}
					Resources of this type are written by every job that uses them, so their values are
					deliberately not kept in history.
				{:else}
					No history yet. Versions are recorded from the next edit onwards.
				{/if}
			</div>
		{:else}
			{#if versions.length > 1 && canClear}
				<div class="px-3 py-2 border-b">
					{#if confirmingClear}
						<p class="text-2xs text-tertiary mb-2">
							Delete every past version, keeping only the current value? Past values can no longer
							be compared or restored.
						</p>
						<div class="flex flex-row gap-2">
							<Button size="xs" variant="accent" disabled={clearing} onclick={clearHistory}>
								{clearing ? 'Clearing' : 'Confirm'}
							</Button>
							<Button size="xs" variant="default" onclick={() => (confirmingClear = false)}>
								Cancel
							</Button>
						</div>
					{:else}
						<Button
							size="xs"
							variant="default"
							startIcon={{ icon: Trash2 }}
							onclick={() => (confirmingClear = true)}
						>
							Clear past versions
						</Button>
					{/if}
				</div>
			{/if}
			<div class="flex flex-col gap-1 p-2">
				{#each versions as version, index (version.id)}
					<VersionListItem
						selected={selectedId === version.id}
						onclick={() => selectVersion(version.id, version.version)}
					>
						<div class="flex flex-col gap-0.5 truncate">
							<div class="text-xs font-medium flex items-center gap-1">
								{#if index === 0}
									<History size={12} />
								{/if}
								{index === 0 ? 'Current' : `Version ${version.version}`}
							</div>
							<div class="text-2xs text-tertiary">
								{displayDate(version.created_at)}{version.created_by
									? ` by ${version.created_by}`
									: ''}
							</div>
						</div>
					</VersionListItem>
				{/each}
			</div>
		{/if}
	</Pane>
	<Pane size={80}>
		{#if loaded === undefined && selectedId !== undefined}
			<Loader2 class="animate-spin m-4" />
		{:else if loaded === undefined}
			<div class="p-4 text-tertiary text-xs">Select a version to inspect its value.</div>
		{:else}
			<div class="flex flex-col h-full">
				<div class="flex flex-row justify-between items-center gap-2 p-2 border-b">
					{#if isCurrent}
						<span class="text-2xs text-tertiary">This is the current value</span>
					{:else if currentValue === undefined}
						<span class="text-2xs text-tertiary">
							The current value could not be loaded, so there is no diff to show
						</span>
					{:else}
						<ToggleButtonGroup bind:selected={view}>
							{#snippet children({ item })}
								<ToggleButton small value="value" icon={Code} label="Value" {item} />
								<ToggleButton small value="diff" icon={Diff} label="Diff with current" {item} />
							{/snippet}
						</ToggleButtonGroup>
					{/if}
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: RotateCcw }}
						disabled={restoring || !canRestore || isCurrent}
						onclick={restore}
					>
						Restore this version
					</Button>
				</div>

				{#if loaded.missing.length > 0}
					<div class="px-3 py-2">
						<Alert type="warning" size="xs" title="References something that no longer exists">
							This version references {loaded.missing.join(', ')}. Restoring it leaves the resource
							pointing at something unresolvable until you recreate it.
						</Alert>
					</div>
				{/if}

				<div class="grow min-h-0">
					{#if view === 'diff' && !isCurrent && currentValue !== undefined}
						<!-- Imported on demand: the diff is the only thing here that needs Monaco, and
						     pulling it in on open would load the editor for everyone who just wants to
						     read a value. -->
						{#await import('./DiffEditor.svelte')}
							<Loader2 class="animate-spin m-4" />
						{:then Module}
							<Module.default
								open
								automaticLayout
								readOnly
								defaultLang="json"
								defaultOriginal={loaded.value}
								defaultModified={currentValue}
								className="h-full"
							/>
						{/await}
					{:else}
						<div class="h-full overflow-auto px-3 py-2">
							<HighlightCode language="json" code={loaded.value} />
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</Pane>
</Splitpanes>
