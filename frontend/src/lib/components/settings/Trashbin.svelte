<script lang="ts">
	import { Button, Skeleton } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { type TrashItem, TrashService } from '$lib/services/trashService'
	import {
		Trash2,
		RotateCcw,
		FileCode2,
		GitFork,
		AppWindow,
		Clock,
		Variable,
		Database,
		Zap,
		RefreshCw
	} from 'lucide-svelte'
	import { untrack } from 'svelte'

	let items: TrashItem[] | undefined = $state(undefined)
	let deleteConfirmedCallback: (() => void) | undefined = $state(undefined)
	let deleteOpen = $derived(Boolean(deleteConfirmedCallback))
	let emptyConfirmOpen = $state(false)

	function getKindIcon(kind: string) {
		if (kind === 'script') return FileCode2
		if (kind === 'flow') return GitFork
		if (kind === 'app') return AppWindow
		if (kind === 'schedule') return Clock
		if (kind === 'variable') return Variable
		if (kind === 'resource') return Database
		if (kind.endsWith('_trigger')) return Zap
		return Trash2
	}

	function getKindLabel(kind: string) {
		if (kind === 'script') return 'Script'
		if (kind === 'flow') return 'Flow'
		if (kind === 'app') return 'App'
		if (kind === 'schedule') return 'Schedule'
		if (kind === 'variable') return 'Variable'
		if (kind === 'resource') return 'Resource'
		if (kind.endsWith('_trigger')) {
			return (
				kind
					.replace('_trigger', '')
					.replace(/_/g, ' ')
					.replace(/\b\w/g, (c) => c.toUpperCase()) + ' Trigger'
			)
		}
		return kind
	}

	function timeAgo(dateStr: string): string {
		const now = new Date()
		const date = new Date(dateStr)
		const diffMs = now.getTime() - date.getTime()
		const diffMins = Math.floor(diffMs / 60000)
		if (diffMins < 1) return 'just now'
		if (diffMins < 60) return `${diffMins}m ago`
		const diffHours = Math.floor(diffMins / 60)
		if (diffHours < 24) return `${diffHours}h ago`
		const diffDays = Math.floor(diffHours / 24)
		return `${diffDays}d ago`
	}

	function timeRemaining(dateStr: string): string {
		const now = new Date()
		const expires = new Date(dateStr)
		const diffMs = expires.getTime() - now.getTime()
		if (diffMs <= 0) return 'expired'
		const diffHours = Math.floor(diffMs / 3600000)
		if (diffHours < 1) return '< 1h remaining'
		if (diffHours < 24) return `${diffHours}h remaining`
		const diffDays = Math.floor(diffHours / 24)
		const remainingHours = diffHours % 24
		if (remainingHours === 0) return `${diffDays}d remaining`
		return `${diffDays}d ${remainingHours}h remaining`
	}

	async function loadItems() {
		items = await TrashService.listTrash({
			workspace: $workspaceStore!
		})
	}

	async function restoreItem(item: TrashItem) {
		try {
			await TrashService.restoreTrashItem({
				workspace: $workspaceStore!,
				id: item.id
			})
			sendUserToast(`Restored ${getKindLabel(item.item_kind)} '${item.item_path}'`)
			loadItems()
		} catch (e) {
			sendUserToast(`Failed to restore: ${e}`, true)
		}
	}

	async function permanentlyDelete(item: TrashItem) {
		try {
			await TrashService.permanentlyDeleteTrashItem({
				workspace: $workspaceStore!,
				id: item.id
			})
			sendUserToast(`Permanently deleted '${item.item_path}'`)
			loadItems()
		} catch (e) {
			sendUserToast(`Failed to delete: ${e}`, true)
		}
	}

	async function emptyAll() {
		try {
			const result = await TrashService.emptyTrash({ workspace: $workspaceStore! })
			sendUserToast(result)
			loadItems()
		} catch (e) {
			sendUserToast(`Failed to empty trash: ${e}`, true)
		}
	}

	$effect(() => {
		$workspaceStore
		untrack(() => loadItems())
	})
</script>

<div class="flex justify-end mb-4 gap-2">
	<Button startIcon={{ icon: RefreshCw }} variant="default" size="xs" onclick={loadItems}>
		Refresh
	</Button>
	<Button
		startIcon={{ icon: Trash2 }}
		variant="default"
		size="xs"
		onclick={() => {
			emptyConfirmOpen = true
		}}
		disabled={!items || items.length === 0}
	>
		Empty Trashbin
	</Button>
</div>

{#if items === undefined}
	<Skeleton layout={[20, 8, 8, 8]} />
{:else if items.length === 0}
	<div class="flex flex-col items-center justify-center py-12 text-tertiary">
		<Trash2 size={40} class="mb-3 opacity-50" />
		<p class="text-base">Trashbin is empty</p>
		<p class="text-sm mt-1">No recently deleted items.</p>
	</div>
{:else}
	<DataTable size="sm">
		{#snippet head()}
			<Head>
				<tr>
					<Cell head first>Type</Cell>
					<Cell head>Path</Cell>
					<Cell head>Deleted by</Cell>
					<Cell head>Deleted</Cell>
					<Cell head>Expires</Cell>
					<Cell head last>Actions</Cell>
				</tr>
			</Head>
		{/snippet}
		{#each items as item (item.id)}
			{@const Icon = getKindIcon(item.item_kind)}
			<Row>
				<Cell first>
					<div class="flex items-center gap-2">
						<Icon size={14} />
						<span class="text-xs">{getKindLabel(item.item_kind)}</span>
					</div>
				</Cell>
				<Cell>
					<span class="font-mono text-xs">{item.item_path}</span>
				</Cell>
				<Cell>
					<span class="text-xs">{item.deleted_by}</span>
				</Cell>
				<Cell>
					<span class="text-xs text-tertiary">{timeAgo(item.deleted_at)}</span>
				</Cell>
				<Cell>
					<span class="text-xs text-tertiary">{timeRemaining(item.expires_at)}</span>
				</Cell>
				<Cell last>
					<div class="flex gap-1">
						<Button
							startIcon={{ icon: RotateCcw }}
							variant="default"
							size="xs2"
							onclick={() => restoreItem(item)}
						>
							Restore
						</Button>
						<Button
							startIcon={{ icon: Trash2 }}
							variant="default"
							size="xs2"
							onclick={() => {
								deleteConfirmedCallback = () => permanentlyDelete(item)
							}}
						>
							Delete
						</Button>
					</div>
				</Cell>
			</Row>
		{/each}
	</DataTable>
{/if}

<ConfirmationModal
	open={deleteOpen}
	title="Permanently delete"
	confirmationText="Delete forever"
	on:canceled={() => {
		deleteConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		if (deleteConfirmedCallback) {
			deleteConfirmedCallback()
		}
		deleteConfirmedCallback = undefined
	}}
>
	<p>This item will be permanently deleted. This action cannot be undone.</p>
</ConfirmationModal>

<ConfirmationModal
	open={emptyConfirmOpen}
	title="Empty trashbin"
	confirmationText="Empty trashbin"
	on:canceled={() => {
		emptyConfirmOpen = false
	}}
	on:confirmed={() => {
		emptyAll()
		emptyConfirmOpen = false
	}}
>
	<p>All items in the trashbin will be permanently deleted. This action cannot be undone.</p>
</ConfirmationModal>
