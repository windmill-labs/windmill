<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import { superadmin, workspaceStore } from '$lib/stores'
	import Notification from '../common/alert/Notification.svelte'
	import List from '../common/layout/List.svelte'
	import { MailboxService, type MailboxMessage } from '$lib/gen'
	import MailboxTable from './MailboxTable.svelte'
	import Select from '../select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import RefreshButton from '../common/button/RefreshButton.svelte'

	interface Props {
		updateUnreadCount?: (count: number) => void
	}

	let { updateUnreadCount = () => {} }: Props = $props()

	let open = $state(false)
	let unreadCount = $state(0)

	let messages = $state<MailboxMessage[]>([])
	let loading = $state(false)

	const typeFilterOptions = [
		{ label: 'All types', value: 'all' },
		{ label: 'Triggers only', value: 'trigger' },
		{ label: 'Debouncing stale data only', value: 'debouncing_stale_data' }
	]

	let selectedTypeFilter = $state('all')
	let showUnreadOnly = $state(false)

	let page = $state(1)
	let pageSize = $state(10)
	let hasMore = $state(true)
	let totalCount = $state(0)

	let refreshInterval: ReturnType<typeof setInterval>
	let isRefreshing = $state(false)

	$effect(() => {
		if (open && $superadmin) {
			refreshMessages()
		}
	})

	onMount(() => {
		refreshInterval = setInterval(() => {
			if (open) {
				refreshMessages(false)
			}
		}, 30000)
	})

	onDestroy(() => {
		if (refreshInterval) {
			clearInterval(refreshInterval)
		}
	})

	async function refreshMessages(resetPage = true) {
		if (!$workspaceStore) return

		if (resetPage) {
			isRefreshing = true
		}

		loading = true
		try {
			const filters: any = {
				workspace: $workspaceStore,
				page: page,
				perPage: pageSize
			}

			if (selectedTypeFilter !== 'all') {
				filters.mailboxType = selectedTypeFilter
			}

			const result = await MailboxService.listMailboxMessages(filters)
			messages = result || []
			hasMore = messages.length === pageSize

			const unread = messages.length
			unreadCount = unread
			totalCount = unread
			updateUnreadCount(unread)
		} catch (error) {
			console.error('Failed to fetch mailbox messages:', error)
			sendUserToast('Failed to fetch mailbox messages', true)
		} finally {
			loading = false
			isRefreshing = false
		}
	}

	async function deleteMessage(messageId: number) {
		try {
			await MailboxService.deleteMailboxMessage({
				workspace: $workspaceStore!,
				messageId
			})
			sendUserToast('Message deleted successfully')
			refreshMessages(false)
		} catch (error) {
			console.error('Failed to delete message:', error)
			sendUserToast('Failed to delete message', true)
		}
	}

	async function bulkDelete(messageIds: number[]) {
		if (messageIds.length === 0) return

		try {
			await MailboxService.bulkDeleteMailboxMessages({
				workspace: $workspaceStore!,
				requestBody: {
					message_ids: messageIds
				}
			})
			sendUserToast(`Deleted ${messageIds.length} messages`)
			refreshMessages(false)
		} catch (error) {
			console.error('Failed to bulk delete messages:', error)
			sendUserToast('Failed to delete messages', true)
		}
	}

	function goToNextPage() {
		if (hasMore) {
			page += 1
			refreshMessages(false)
		}
	}

	function goToPreviousPage() {
		if (page > 1) {
			page -= 1
			refreshMessages(false)
		}
	}

	$effect(() => {
		if (selectedTypeFilter !== undefined && showUnreadOnly !== undefined) {
			refreshMessages(true)
		}
	})

	export function openModal() {
		open = true
	}

	export function closeModal() {
		open = false
	}

	export function toggleModal() {
		if (open) {
			closeModal()
		} else {
			openModal()
		}
	}
</script>

{#if $superadmin}
	<Modal2 bind:isOpen={open} title="Mailbox" target="#content" fixedSize="lg">
		<svelte:fragment slot="header-left">
			<Notification notificationCount={unreadCount} notificationLimit={9999} />
		</svelte:fragment>

		<List gap="sm">
			<div class="w-full">
				<List horizontal justify="between">
					<div class="w-full">
						<List horizontal justify="start" gap="md">
							<div class="min-w-48">
								<Select
									items={typeFilterOptions}
									bind:value={selectedTypeFilter}
									placeholder="Filter by type"
									size="sm"
								/>
							</div>
						</List>
					</div>

					<List wFull={false} horizontal gap="md" justify="end">
						<div class="text-xs text-primary whitespace-nowrap">
							{`${totalCount >= 1000 ? '1000+' : (totalCount ?? '?')} items`}
						</div>
						<RefreshButton loading={isRefreshing} on:click={() => refreshMessages(true)} />
					</List>
				</List>
			</div>

			<MailboxTable
				{messages}
				{loading}
				{deleteMessage}
				{bulkDelete}
				{goToNextPage}
				{goToPreviousPage}
				{showUnreadOnly}
				bind:page
				{hasMore}
				{pageSize}
			/>
		</List>
	</Modal2>
{/if}
