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
	import { AlertTriangle, Code, Diff, History, RotateCcw } from 'lucide-svelte'
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
	let selectedId = $state<number | undefined>(undefined)
	let selectedValue = $state<string | undefined>(undefined)
	let missingReferences = $state<string[]>([])
	let currentValue = $state<string>('')
	let view = $state<'value' | 'diff'>('value')
	let restoring = $state(false)

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
		if (id === undefined) {
			selectedId = undefined
			selectedValue = undefined
			missingReferences = []
			return
		}
		selectedId = id
		const version = await ResourceService.getResourceVersion({
			workspace: effectiveWorkspace,
			version: id
		})
		selectedValue = pretty(version.value)
		missingReferences = version.missing_references ?? []
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
		{#if selectedValue === undefined}
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
						disabled={restoring || versions?.[0]?.id === selectedId}
						onclick={restore}
					>
						Restore this version
					</Button>
				</div>

				{#if missingReferences.length > 0}
					<div
						class="flex flex-row gap-2 items-start text-2xs text-orange-600 dark:text-orange-400 px-3 py-2 border-b"
					>
						<AlertTriangle size={14} class="shrink-0 mt-0.5" />
						<div>
							This version references {missingReferences.join(', ')}, which no longer exists.
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
							defaultOriginal={selectedValue}
							defaultModified={currentValue}
							className="h-full"
						/>
					{:else}
						<SimpleEditor
							class="h-full"
							lang="json"
							bind:code={selectedValue}
							readOnly
							automaticLayout
						/>
					{/if}
				</div>
			</div>
		{/if}
	</Pane>
</Splitpanes>
