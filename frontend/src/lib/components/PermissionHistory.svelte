<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'
	import TableCustom from './TableCustom.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Label from './Label.svelte'

	interface PermissionChange {
		id?: number
		changed_by?: string
		changed_at?: string
		change_type?: string
		owner_affected?: string | null
		member_affected?: string | null
	}

	interface Props {
		name: string
		kind: 'folder' | 'group'
		fetchHistory: (
			workspace: string,
			name: string,
			page: number,
			perPage: number
		) => Promise<PermissionChange[]>
	}

	let { name, kind, fetchHistory }: Props = $props()
	let history: PermissionChange[] | undefined = $state(undefined)
	let loading = $state(false)
	let page = $state(1)
	let perPage = $state(10)

	async function loadHistory() {
		if (!$workspaceStore) return
		loading = true
		try {
			history = await fetchHistory($workspaceStore, name, page, perPage)
		} catch (e) {
			console.error('Failed to load permission history:', e)
			history = []
		} finally {
			loading = false
		}
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr)
		return date.toLocaleString()
	}

	function formatChangeType(changeType: string): string {
		return changeType
			.split('_')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ')
	}

	$effect.pre(() => {
		if ($workspaceStore && name) {
			untrack(() => {
				loadHistory()
			})
		}
	})
</script>

<Label label="Permission History">
	{#if loading || history === undefined}
		<div class="flex flex-col gap-2">
			{#each new Array(3) as _}
				<Skeleton layout={[[4], 0.7]} />
			{/each}
		</div>
	{:else if history.length === 0}
		<p class="text-primary text-sm">No permission changes recorded yet</p>
	{:else}
		<TableCustom>
			<tr slot="header-row">
				<th>Changed By</th>
				<th>Change Type</th>
				<th>{kind === 'folder' ? 'Owner Affected' : 'Member Affected'}</th>
				<th>Date</th>
			</tr>
			{#snippet body()}
				<tbody>
					{#each history as change}
						<tr>
							<td>{change.changed_by ?? '-'}</td>
							<td>{change.change_type ? formatChangeType(change.change_type) : '-'}</td>
							<td>{change.owner_affected ?? change.member_affected ?? '-'}</td>
							<td class="text-xs">{change.changed_at ? formatDate(change.changed_at) : '-'}</td>
						</tr>
					{/each}
				</tbody>
			{/snippet}
		</TableCustom>
	{/if}
</Label>
