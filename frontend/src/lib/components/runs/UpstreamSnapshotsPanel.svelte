<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { ClipboardCopy } from 'lucide-svelte'

	// Shape produced by asset-dispatch (backend asset_dispatch.rs
	// `UpstreamSnapshot`): the substrate version each direct upstream asset was
	// at when this job was dispatched. Forensic record only — the run's reads
	// are not pinned to these versions.
	type UpstreamSnapshot = { asset: string; snapshot_id: number; partition?: string }

	type Props = { args: any }
	let { args }: Props = $props()

	const snapshots: UpstreamSnapshot[] = $derived.by(() => {
		const trigger = args?.trigger
		if (trigger?.kind !== 'asset' || !Array.isArray(trigger?.upstream_snapshots)) return []
		// args are caller-supplied JSON, so dedupe by asset — a crafted
		// duplicate would otherwise crash the keyed each below.
		const seen = new Set<string>()
		return trigger.upstream_snapshots.filter((s: any) => {
			if (typeof s?.asset !== 'string' || typeof s?.snapshot_id !== 'number') return false
			if (seen.has(s.asset)) return false
			seen.add(s.asset)
			return true
		})
	})
</script>

{#if snapshots.length > 0}
	<div class="mr-2 sm:mr-0 mt-12 mb-6">
		<h3 class="text-xs font-semibold text-emphasis mb-1">
			Upstream snapshots
			<Tooltip>
				Version of each upstream asset when this run was dispatched. Recorded for debugging only —
				the run reads the latest data, not these versions. To inspect what this run saw, query the
				asset with the copied <span class="font-mono">AT (VERSION =&gt; n)</span> clause. For partitioned
				assets, the partition shown is the slice whose write produced that snapshot — the snapshot itself
				covers the whole table.
			</Tooltip>
		</h3>
		<div class="border rounded-md overflow-hidden">
			<table class="w-full text-xs">
				<thead class="bg-surface-secondary text-secondary">
					<tr>
						<th class="text-left font-normal px-3 py-2">Asset</th>
						<th class="text-left font-normal px-3 py-2">Snapshot</th>
						<th class="text-right font-normal px-3 py-2">Time travel</th>
					</tr>
				</thead>
				<tbody>
					{#each snapshots as s (s.asset)}
						<tr class="border-t">
							<td class="px-3 py-2 align-top">
								<span class="font-mono">{s.asset}</span>
							</td>
							<td class="px-3 py-2 align-top whitespace-nowrap">
								<span class="font-mono">@ {s.snapshot_id}</span>
								{#if s.partition}
									<span class="text-secondary">
										· partition <span class="font-mono">{s.partition}</span></span
									>
								{/if}
							</td>
							<td class="px-3 py-2 align-top text-right whitespace-nowrap">
								<Button
									variant="subtle"
									unifiedSize="sm"
									startIcon={{ icon: ClipboardCopy }}
									onclick={() => copyToClipboard(`AT (VERSION => ${s.snapshot_id})`)}
								>
									AT (VERSION =&gt; {s.snapshot_id})
								</Button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>
{/if}
