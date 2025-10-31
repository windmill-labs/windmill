<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Mail, MailOpen, Trash2, Eye } from 'lucide-svelte'
	import List from '$lib/components/common/layout/List.svelte'
	import { Skeleton } from '../common'
	import type { MailboxMessage } from '$lib/gen'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	interface Props {
		messages: MailboxMessage[]
		loading?: boolean
		showUnreadOnly?: boolean
		goToNextPage: () => void
		goToPreviousPage: () => void
		deleteMessage: (id: number) => void
		bulkDelete: (ids: number[]) => void
		page?: number
		hasMore?: boolean
		pageSize?: number
	}

	let {
		messages,
		loading = false,
		showUnreadOnly: _showUnreadOnly = false,
		goToNextPage,
		goToPreviousPage,
		deleteMessage,
		bulkDelete,
		page = $bindable(1),
		hasMore = true,
		pageSize = 10
	}: Props = $props()

	let headerHeight = $state(0)
	let contentHeight = $state(0)
	let selectedMessages = $state(new Set<number>())
	let allSelected = $state(false)

	let payloadModalOpen = $state(false)
	let selectedPayload = $state<any>(null)
	let selectedMessageId = $state<number | null>(null)

	function formatDate(dateString: string): string {
		const date = new Date(dateString)
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		}).format(date)
	}

	function openPayloadViewer(messageId: number, payload: any, event?: Event) {
		if (event) {
			event.stopPropagation()
		}
		selectedMessageId = messageId
		selectedPayload = payload
		payloadModalOpen = true
	}

	function handleBulkDelete() {
		if (selectedMessages.size > 0) {
			bulkDelete(Array.from(selectedMessages))
			selectedMessages.clear()
			allSelected = false
		}
	}

	$effect(() => {
		if (messages) {
			selectedMessages.clear()
			allSelected = false
			selectedMessages = selectedMessages
		}
	})

	const availableHeight = $derived((contentHeight - headerHeight - pageSize - 1) / pageSize)
</script>

<div class="relative grow min-h-0 w-full">
	<DataTable
		size="xs"
		paginated
		on:next={goToNextPage}
		on:previous={goToPreviousPage}
		bind:currentPage={page}
		{hasMore}
		bind:contentHeight
	>
		<Head>
			<tr bind:clientHeight={headerHeight}>
				<Cell head class="max-w-4">&nbsp;</Cell>
				<Cell head class="min-w-20">ID</Cell>
				<Cell head class="min-w-24">Type</Cell>
				<Cell head class="min-w-32">Mailbox ID</Cell>
				<Cell head class="min-w-28">Created At</Cell>
				<Cell head last class="min-w-40">
					<List horizontal gap="sm">
						<span>Actions</span>
						{#if selectedMessages.size > 0}
							<Button
								color="red"
								startIcon={{ icon: Trash2 }}
								size="xs2"
								on:click={handleBulkDelete}
								title="Delete selected"
							>
								Delete ({selectedMessages.size})
							</Button>
						{/if}
					</List>
				</Cell>
			</tr>
		</Head>
		{#if loading}
			<tbody>
				{#each new Array(pageSize) as _}
					<Row>
						{#each new Array(7) as _}
							<Cell>
								<Skeleton layout={[[5]]} />
							</Cell>
						{/each}
					</Row>
				{/each}
			</tbody>
		{:else if messages.length === 0}
			<div class="absolute top-0 left-0 w-full h-full center-center">
				<div class="text-center text-secondary">
					<Mail size="48" class="mx-auto mb-3 text-tertiary" />
					<h3 class="font-medium mb-1">No messages</h3>
					<p class="text-sm text-tertiary">No mailbox messages found.</p>
				</div>
			</div>
		{:else}
			<tbody class="divide-y border-b w-full overflow-y-auto">
				{#each messages as message (message.message_id)}
					<Row>
						<Cell class="py-0">
							<div class="flex items-center justify-center" style="height: {availableHeight}px">
								{#if message.type === 'trigger'}
									<span title="Trigger Message">
										<Mail size="16" class="text-blue-500" />
									</span>
								{:else}
									<span title="Debouncing Stale Data">
										<MailOpen size="16" class="text-amber-500" />
									</span>
								{/if}
							</div>
						</Cell>

						<Cell wrap>
							<span class="font-mono text-sm">{message.message_id}</span>
						</Cell>

						<Cell wrap>
							{#if message.type === 'trigger'}
								<span
									class="px-2 py-1 rounded text-xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300"
								>
									{message.type}
								</span>
							{:else}
								<span
									class="px-2 py-1 rounded text-xs font-medium bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300"
								>
									{message.type}
								</span>
							{/if}
						</Cell>

						<Cell wrap>
							<span class="font-mono text-sm text-tertiary">
								{message.mailbox_id || '—'}
							</span>
						</Cell>

						<Cell wrap>
							<span class="text-sm text-tertiary">
								{formatDate(message.created_at)}
							</span>
						</Cell>

						<Cell>
							<div class="w-full flex justify-center items-center">
								<List horizontal gap="sm">
									<Button
										color="light"
										size="xs2"
										startIcon={{ icon: Eye }}
										on:click={(e) => openPayloadViewer(message.message_id, message.payload, e)}
										title="View full payload"
									>
										Payload
									</Button>
									<Button
										color="red"
										startIcon={{ icon: Trash2 }}
										size="xs2"
										on:click={() => deleteMessage(message.message_id)}
										title="Delete message"
									>
										Delete
									</Button>
								</List>
							</div>
						</Cell>
					</Row>
				{/each}
			</tbody>
		{/if}
	</DataTable>
</div>

<Modal2 bind:isOpen={payloadModalOpen} title="Message Payload" fixedSize="xl">
	{#if selectedPayload !== null}
		<div class="flex flex-col space-y-4">
			<div class="text-sm text-secondary">
				Message ID: <span class="font-mono text-primary">{selectedMessageId}</span>
			</div>
			<div style="height: 500px;">
				<SimpleEditor
					lang="json"
					code={JSON.stringify(selectedPayload, null, 2)}
					class="border rounded-md h-full"
					fixedOverflowWidgets={false}
					disabled={true}
					automaticLayout={true}
					autoHeight={false}
				/>
			</div>
		</div>
	{/if}
</Modal2>
