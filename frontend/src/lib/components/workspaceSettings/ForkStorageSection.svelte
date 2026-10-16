<script lang="ts">
	import { SettingService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import { TriangleAlert } from 'lucide-svelte'

	// The workspace the fork is created from, like ForkDucklakeSection: the storages
	// listed here must match the ones the create-fork call submits to.
	let {
		sourceWorkspace,
		requireIsolation = false,
		itemCount = $bindable()
	}: { sourceWorkspace?: string; requireIsolation?: boolean; itemCount?: number } = $props()
	let effectiveSource = $derived(sourceWorkspace ?? $workspaceStore ?? undefined)

	let storages = resource(
		() => effectiveSource,
		async (ws) =>
			ws
				? SettingService.getSecondaryStorageNames({ workspace: ws, includeDefault: true })
				: undefined
	)

	$effect(() => {
		itemCount = storages.current?.length ?? 0
	})

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
	<div class="flex flex-col gap-1">
		<span class="text-xs font-medium text-secondary">Object storage</span>
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
							{ value: 'isolated', label: 'Isolated' },
							{ value: 'shared', label: 'Shared with parent' }
						]}
					/>
				</div>
			{/each}
		</div>
		{#if anyShared}
			<span class="flex items-center gap-1 text-2xs text-amber-700 dark:text-amber-300">
				<TriangleAlert size={12} class="shrink-0" />
				Scripts in the fork will overwrite and delete this workspace's files.
			</span>
		{/if}
	</div>
{/if}
