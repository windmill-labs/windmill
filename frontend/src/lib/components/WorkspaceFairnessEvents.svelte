<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { Section } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Badge from './common/badge/Badge.svelte'
	import Alert from './common/alert/Alert.svelte'
	import Button from './common/button/Button.svelte'
	import { displayDate } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'

	type FairnessEvent = {
		timestamp: string
		operation: string
		workspace_id?: string | null
		parameters?: Record<string, unknown> | null
	}

	let events: FairnessEvent[] = $state([])
	let loading = $state(true)
	let error: string | undefined = $state(undefined)

	async function load() {
		loading = true
		error = undefined
		try {
			const res = await WorkerService.getWorkspaceFairnessEvents()
			events = (res as unknown as FairnessEvent[]) ?? []
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		} finally {
			loading = false
		}
	}

	load()

	function formatParameters(p?: Record<string, unknown> | null): string {
		if (!p) return ''
		// Audit-log params arrive as `{ hm: { ... }, args: ... }` in EE format —
		// surface the `hm` map when present, otherwise show the object as-is.
		const inner = (p as { hm?: Record<string, unknown> }).hm ?? p
		return Object.entries(inner)
			.map(([k, v]) => `${k}=${v}`)
			.join(', ')
	}
</script>

<Section label="Workspace fairness events">
	{#snippet action()}
		<Button
			variant="subtle"
			size="xs"
			startIcon={{ icon: RefreshCw }}
			iconOnly
			onclick={load}
			disabled={loading}
		/>
	{/snippet}

	{#if loading}
		<Skeleton layout={[[8]]} />
	{:else if error}
		<Alert type="error" title="Failed to load">{error}</Alert>
	{:else if events.length === 0}
		<p class="text-secondary text-sm">
			No workspaces have been added to or removed from the fairness-restricted set yet. Entries
			appear here once the cap activates.
		</p>
	{:else}
		<div class="text-xs text-secondary mb-2">
			Showing the last {events.length} workspace-fairness transitions (most recent first).
		</div>
		<div class="overflow-x-auto border rounded-md">
			<table class="min-w-full text-sm">
				<thead class="bg-surface-secondary text-secondary">
					<tr>
						<th class="text-left px-3 py-2 font-medium">Time</th>
						<th class="text-left px-3 py-2 font-medium">Event</th>
						<th class="text-left px-3 py-2 font-medium">Workspace</th>
						<th class="text-left px-3 py-2 font-medium">Details</th>
					</tr>
				</thead>
				<tbody>
					{#each events as e}
						<tr class="border-t">
							<td class="px-3 py-2 whitespace-nowrap text-tertiary">
								{displayDate(e.timestamp, true)}
							</td>
							<td class="px-3 py-2 whitespace-nowrap">
								{#if e.operation === 'workspace_fairness.capped'}
									<Badge color="red">capped</Badge>
								{:else if e.operation === 'workspace_fairness.uncapped'}
									<Badge color="green">uncapped</Badge>
								{:else}
									<Badge>{e.operation}</Badge>
								{/if}
							</td>
							<td class="px-3 py-2 font-mono">{e.workspace_id ?? '—'}</td>
							<td class="px-3 py-2 text-tertiary text-xs">{formatParameters(e.parameters)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</Section>
