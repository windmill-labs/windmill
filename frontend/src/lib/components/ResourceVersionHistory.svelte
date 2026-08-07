<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ResourceService, type ResourceVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import Button from './common/button/Button.svelte'
	import DiffEditor from './DiffEditor.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { AlertTriangle, Code, Diff, History, RotateCcw, Trash2 } from 'lucide-svelte'
	import { displayDate } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'

	let {
		path,
		workspace = undefined,
		onRestore = undefined
	}: {
		path: string
		workspace?: string
		onRestore?: () => void
	} = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	let versions = $state<ResourceVersion[] | undefined>(undefined)
	// Which row is highlighted, updated on click. `loaded` lags it while the value is in
	// flight, and the editor renders from `loaded` so it can never show one version's JSON
	// under another version's label.
	let selectedId = $state<number | undefined>(undefined)
	let loaded = $state<{ id: number; value: string; missing: string[] } | undefined>(undefined)
	let currentValue = $state<string>('')
	let view = $state<'value' | 'diff'>('value')
	let restoring = $state(false)
	let clearing = $state(false)
	let confirmingClear = $state(false)

	function pretty(value: unknown): string {
		return JSON.stringify(value ?? null, null, 2)
	}

	async function loadVersions() {
		versions = undefined
		const [list, current] = await Promise.all([
			ResourceService.getResourceHistory({ workspace: effectiveWorkspace, path }),
			ResourceService.getResourceValue({ workspace: effectiveWorkspace, path }).catch(() => null)
		])
		currentValue = pretty(current)
		versions = list
		// The newest version mirrors the live value, so the first entry a user can usefully
		// compare against is the one below it.
		await selectVersion(list[1]?.id ?? list[0]?.id)
	}

	async function selectVersion(id: number | undefined) {
		selectedId = id
		if (id === undefined) {
			loaded = undefined
			return
		}
		const version = await ResourceService.getResourceVersion({
			workspace: effectiveWorkspace,
			version: id
		})
		// Assigned as one object so the id keying the editor and the value it displays can
		// never be out of step, whatever order the requests come back in.
		if (selectedId === id) {
			loaded = { id, value: pretty(version.value), missing: version.missing_references ?? [] }
		}
	}

	async function restore() {
		if (selectedId === undefined) return
		restoring = true
		try {
			await ResourceService.restoreResourceVersion({
				workspace: effectiveWorkspace,
				version: selectedId
			})
			sendUserToast(`Restored ${path} to version ${selectedId}`)
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
	<Pane size={25} minSize={20}>
		{#if versions === undefined}
			<div class="p-2 flex flex-col gap-2">
				{#each Array(4) as _}
					<Skeleton layout={[[2]]} />
				{/each}
			</div>
		{:else if versions.length === 0}
			<div class="p-4 text-tertiary text-xs">
				No history yet. Versions are recorded from the next edit onwards.
			</div>
		{:else}
			{#if versions.length > 1}
				<div class="px-3 py-2 border-b">
					{#if confirmingClear}
						<p class="text-2xs text-tertiary mb-2">
							Delete every past version, keeping only the current value? Past values can no
							longer be compared or restored.
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
			<ul class="divide-y">
				{#each versions as version, index (version.id)}
					<li>
						<button
							class="w-full text-left px-3 py-2 hover:bg-surface-hover {selectedId === version.id
								? 'bg-surface-selected'
								: ''}"
							onclick={() => selectVersion(version.id)}
						>
							<div class="text-xs font-medium flex items-center gap-1">
								{#if index === 0}
									<History size={12} />
								{/if}
								{index === 0 ? 'Current' : `Version ${version.id}`}
							</div>
							<div class="text-2xs text-tertiary">
								{displayDate(version.created_at)}{version.created_by
									? ` by ${version.created_by}`
									: ''}
							</div>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</Pane>
	<Pane size={75}>
		{#if loaded === undefined}
			<div class="p-4 text-tertiary text-xs">Select a version to inspect its value.</div>
		{:else}
			<div class="flex flex-col h-full">
				<div class="flex flex-row justify-between items-center gap-2 p-2 border-b">
					<ToggleButtonGroup bind:selected={view}>
						{#snippet children({ item })}
							<ToggleButton small value="value" icon={Code} label="Value" {item} />
							<ToggleButton small value="diff" icon={Diff} label="Diff with current" {item} />
						{/snippet}
					</ToggleButtonGroup>
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: RotateCcw }}
						disabled={restoring || versions?.[0]?.id === loaded.id}
						onclick={restore}
					>
						Restore this version
					</Button>
				</div>

				{#if loaded.missing.length > 0}
					<div
						class="flex flex-row gap-2 items-start text-2xs text-orange-600 dark:text-orange-400 px-3 py-2 border-b"
					>
						<AlertTriangle size={14} class="shrink-0 mt-0.5" />
						<div>
							This version references {loaded.missing.join(', ')}, which no longer exists.
							Restoring it will leave the resource pointing at something unresolvable until you
							recreate it.
						</div>
					</div>
				{/if}

				<div class="grow min-h-0">
					{#if view === 'diff'}
						<DiffEditor
							open
							automaticLayout
							readOnly
							defaultLang="json"
							defaultOriginal={loaded.value}
							defaultModified={currentValue}
							className="h-full"
						/>
					{:else}
						<!-- Keyed on the version: SimpleEditor reads `code` only when it builds its
						     Monaco model and has no effect syncing later changes, so an already-mounted
						     editor would keep showing the first version opened while Restore acted on
						     the one actually selected. -->
						{#key loaded.id}
							<SimpleEditor
								class="h-full"
								lang="json"
								code={loaded.value}
								readOnly
								automaticLayout
							/>
						{/key}
					{/if}
				</div>
			</div>
		{/if}
	</Pane>
</Splitpanes>
