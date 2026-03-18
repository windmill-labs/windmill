<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import type { QuotaInfo } from '$lib/gen'
	import { untrack } from 'svelte'
	import { Trash2 } from 'lucide-svelte'

	type ResourceType = 'scripts' | 'flows' | 'apps'

	let quotas:
		| {
				scripts: QuotaInfo
				flows: QuotaInfo
				apps: QuotaInfo
				variables: QuotaInfo
				resources: QuotaInfo
		  }
		| undefined = $state(undefined)

	let loading = $state(false)
	let pruning = $state(false)
	let drawer: Drawer | undefined = $state()
	let pruneTarget: ResourceType | undefined = $state(undefined)

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadQuotas())
		}
	})

	async function loadQuotas() {
		loading = true
		try {
			quotas = await WorkspaceService.getCloudQuotas({ workspace: $workspaceStore! })
		} catch (e) {
			sendUserToast(`Failed to load cloud quotas: ${e}`, true)
		} finally {
			loading = false
		}
	}

	function openPruneDrawer(type: ResourceType) {
		pruneTarget = type
		drawer?.openDrawer()
	}

	async function confirmPrune() {
		if (!pruneTarget) return
		pruning = true
		try {
			const result = await WorkspaceService.pruneVersions({
				workspace: $workspaceStore!,
				requestBody: { resource_type: pruneTarget }
			})
			sendUserToast(`Pruned ${result.pruned} old ${pruneTarget} versions`)
			drawer?.closeDrawer()
			await loadQuotas()
		} catch (e) {
			sendUserToast(`Failed to prune: ${e}`, true)
		} finally {
			pruning = false
		}
	}

	function getPrunableCount(type: ResourceType): number {
		if (!quotas) return 0
		return quotas[type].prunable
	}

	function getPruneDescription(type: ResourceType): string {
		switch (type) {
			case 'scripts':
				return 'This will permanently delete all non-HEAD script versions (old edits). The latest deployed version of each script will be preserved. This directly frees up quota space since each script edit creates a new row counted against the limit.'
			case 'flows':
				return 'This will permanently delete all non-HEAD flow versions. Only the latest version of each flow will be kept. This frees up storage but does not reduce the flow count (quota counts unique flows, not versions).'
			case 'apps':
				return 'This will permanently delete all non-HEAD app versions. Only the latest version of each app will be kept. This frees up storage but does not reduce the app count (quota counts unique apps, not versions).'
		}
	}

	const rows: { label: string; key: keyof NonNullable<typeof quotas>; prunable: boolean }[] = [
		{ label: 'Scripts', key: 'scripts', prunable: true },
		{ label: 'Flows', key: 'flows', prunable: true },
		{ label: 'Apps', key: 'apps', prunable: true },
		{ label: 'Variables', key: 'variables', prunable: false },
		{ label: 'Resources', key: 'resources', prunable: false }
	]
</script>

<div class="flex flex-col gap-2">
	<p class="font-semibold text-xs text-emphasis">Cloud Quotas</p>
	<p class="text-xs text-secondary font-normal">
		Current usage and limits for this workspace. Prune old versions to free up space.
	</p>

	{#if loading && !quotas}
		<p class="text-xs text-tertiary">Loading...</p>
	{:else if quotas}
		<div class="border rounded-md overflow-hidden">
			<table class="w-full text-xs">
				<thead>
					<tr class="bg-surface-secondary border-b">
						<th class="text-left px-3 py-2 text-secondary font-medium">Resource</th>
						<th class="text-left px-3 py-2 text-secondary font-medium">Usage</th>
						<th class="text-right px-3 py-2 text-secondary font-medium">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as row (row.key)}
						{@const info = quotas[row.key]}
						<tr class="border-b last:border-b-0">
							<td class="px-3 py-2 text-primary font-medium">{row.label}</td>
							<td class="px-3 py-2">
								<span
									class={info.used >= info.limit ? 'text-red-500 font-semibold' : 'text-primary'}
								>
									{info.used}
								</span>
								<span class="text-tertiary">/ {info.limit}</span>
							</td>
							<td class="px-3 py-2 text-right">
								{#if row.prunable && info.prunable > 0}
									<Button
										unifiedSize="sm"
										variant="default"
										startIcon={{ icon: Trash2 }}
										on:click={() => openPruneDrawer(row.key as ResourceType)}
									>
										Prune {info.prunable} old versions
									</Button>
								{:else if row.prunable}
									<span class="text-tertiary">No old versions</span>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<Drawer bind:this={drawer}>
	<DrawerContent title="Prune old versions" on:close={drawer?.closeDrawer}>
		{#if pruneTarget}
			<div class="flex flex-col gap-4">
				<p class="text-sm text-primary">
					You are about to prune <span class="font-semibold">{getPrunableCount(pruneTarget)}</span>
					old {pruneTarget} versions.
				</p>
				<p class="text-xs text-secondary">
					{getPruneDescription(pruneTarget)}
				</p>
				<p class="text-xs text-red-500 font-medium">This action cannot be undone.</p>
			</div>
		{/if}
		{#snippet actions()}
			<Button variant="accent" on:click={confirmPrune} disabled={pruning}>
				{pruning ? 'Pruning...' : 'Confirm Prune'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
