<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import Label from '../Label.svelte'
	import { TriangleAlert } from 'lucide-svelte'

	// Keyed on the store so a direct page load (store hydrates after mount) still fetches.
	let allDucklakes = resource(
		() => $workspaceStore,
		async (ws) => (ws ? WorkspaceService.listDucklakes({ workspace: ws }) : undefined)
	)

	let lakeBehaviors: Record<string, 'isolated' | 'shared'> = $state({})

	// Lakes the user explicitly opted out of isolation for — sent as
	// `shared_ducklakes` on the create-fork request. Unlisted lakes default to the
	// isolated fork namespace with read-defer, so nothing needs sending for them.
	export function getSharedDucklakes(): string[] {
		return (allDucklakes.current ?? []).filter((name) => lakeBehaviors[name] === 'shared')
	}

	let anyShared = $derived((allDucklakes.current ?? []).some((n) => lakeBehaviors[n] === 'shared'))
</script>

<!-- Self-hides when the workspace has no ducklake configured, like the datatable section. -->
{#if allDucklakes.current && allDucklakes.current.length > 0}
	<Label label="Ducklake data environment">
		<span class="text-xs text-secondary">
			By default each lake is isolated in the fork: materializations write a fork-scoped namespace
			and reads of not-yet-materialized tables defer to this workspace's current data.
		</span>
		<div class="border rounded-md divide-y">
			{#each allDucklakes.current as name (name)}
				<div class="flex items-center gap-2 justify-between px-4 py-1.5">
					<span class="text-xs font-medium">{name}</span>
					<Select
						dropdownClass="max-w-96"
						bind:value={() => lakeBehaviors[name] ?? 'isolated', (v) => (lakeBehaviors[name] = v)}
						items={[
							{ value: 'isolated', label: 'Isolated (defer reads to parent)' },
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
					A shared lake is NOT isolated: pipeline runs in the fork read and write this workspace's
					tables directly.
				</span>
			</div>
		{/if}
	</Label>
{/if}
