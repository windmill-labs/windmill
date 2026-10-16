<script lang="ts">
	import { SettingService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import Label from '../Label.svelte'
	import { TriangleAlert } from 'lucide-svelte'

	// The workspace the fork is created from, like ForkDucklakeSection: the storages
	// listed here must match the ones the create-fork call submits to.
	let {
		sourceWorkspace,
		requireIsolation = false
	}: { sourceWorkspace?: string; requireIsolation?: boolean } = $props()
	let effectiveSource = $derived(sourceWorkspace ?? $workspaceStore ?? undefined)

	let storages = resource(
		() => effectiveSource,
		async (ws) =>
			ws
				? SettingService.getSecondaryStorageNames({ workspace: ws, includeDefault: true })
				: undefined
	)

	let behaviors: Record<string, 'isolated' | 'shared'> = $state({})

	// Sent as `shared_storages` on the create-fork request; unlisted storages are isolated.
	export function getSharedStorages(): string[] {
		if (requireIsolation) return []
		return (storages.current ?? []).filter((name) => behaviors[name] === 'shared')
	}

	let anyShared = $derived(
		!requireIsolation && (storages.current ?? []).some((n) => behaviors[n] === 'shared')
	)

	function storageLabel(name: string): string {
		return name === '_default_' ? 'Primary storage' : name
	}
</script>

<!-- Self-hides when the workspace has no object storage configured. -->
{#if storages.current && storages.current.length > 0}
	<Label label="Object storage">
		<span class="text-xs text-secondary">
			By default each storage is isolated in the fork: the fork's writes go to its own prefix, and
			it reads this workspace's files until it writes its own. Deleting the fork deletes what it
			wrote.
			{#if requireIsolation}
				A protection rule on this workspace requires its forks to keep every storage isolated.
			{/if}
		</span>
		<div class="border rounded-md divide-y">
			{#each storages.current as name (name)}
				<div class="flex items-center gap-2 justify-between px-4 py-1.5">
					<span class="text-xs font-medium">{storageLabel(name)}</span>
					<Select
						dropdownClass="max-w-96"
						disabled={requireIsolation}
						bind:value={
							() => (requireIsolation ? 'isolated' : (behaviors[name] ?? 'isolated')),
							(v) => (behaviors[name] = v)
						}
						items={[
							{ value: 'isolated', label: 'Isolated (read through to parent)' },
							{ value: 'shared', label: 'Shared with parent' }
						]}
					/>
				</div>
			{/each}
		</div>
		{#if anyShared}
			<div
				class="flex items-start gap-2 px-3 py-2 text-xs rounded-md border border-amber-200 dark:border-amber-900/60 bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300"
			>
				<TriangleAlert size={14} class="shrink-0 mt-0.5" />
				<span>
					A shared storage is NOT isolated: scripts in the fork read, overwrite and delete this
					workspace's files directly.
				</span>
			</div>
		{/if}
	</Label>
{/if}
