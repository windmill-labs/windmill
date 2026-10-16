<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import { TriangleAlert } from 'lucide-svelte'

	// The workspace the fork is created from (matches ForkDatatableSection): a
	// fork-of-fork bases off the selected base, not the navigation workspace, so
	// the lakes listed here must match the ones the create-fork call submits to.
	let {
		sourceWorkspace,
		requireIsolation = false,
		itemCount = $bindable()
	}: { sourceWorkspace?: string; requireIsolation?: boolean; itemCount?: number } = $props()
	let effectiveSource = $derived(sourceWorkspace ?? $workspaceStore ?? undefined)

	// Keyed on the source so a direct page load (store hydrates after mount) still fetches.
	let allDucklakes = resource(
		() => effectiveSource,
		async (ws) => (ws ? WorkspaceService.listDucklakes({ workspace: ws }) : undefined)
	)

	$effect(() => {
		itemCount = allDucklakes.current?.length ?? 0
	})

	let lakeBehaviors: Record<string, 'isolated' | 'shared'> = $state({})

	// Lakes the user explicitly opted out of isolation for — sent as
	// `shared_ducklakes` on the create-fork request. Unlisted lakes default to the
	// isolated fork namespace with read-defer, so nothing needs sending for them.
	export function getSharedDucklakes(): string[] {
		if (requireIsolation) return []
		return (allDucklakes.current ?? []).filter((name) => lakeBehaviors[name] === 'shared')
	}

	let anyShared = $derived(
		!requireIsolation && (allDucklakes.current ?? []).some((n) => lakeBehaviors[n] === 'shared')
	)
</script>

<!-- Self-hides when the workspace has no ducklake configured, like the datatable section. -->
{#if allDucklakes.current && allDucklakes.current.length > 0}
	<div class="flex flex-col gap-1">
		<span class="text-xs font-medium text-secondary">Ducklakes</span>
		<div class="border rounded-md divide-y">
			{#each allDucklakes.current as name (name)}
				<div class="flex items-center gap-2 justify-between px-4 py-1.5">
					<span class="text-xs font-medium">{name}</span>
					<Select
						dropdownClass="max-w-96"
						disabled={requireIsolation}
						bind:value={
							() => (requireIsolation ? 'isolated' : (lakeBehaviors[name] ?? 'isolated')),
							(v) => (lakeBehaviors[name] = v)
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
				Pipeline runs in the fork will write this workspace's tables.
			</span>
		{/if}
	</div>
{/if}
