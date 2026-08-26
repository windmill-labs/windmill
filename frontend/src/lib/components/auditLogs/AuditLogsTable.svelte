<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { AuditLog } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { onMount, tick } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { ChevronLeft, ChevronRight, ListFilterPlus, Loader2 } from 'lucide-svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import { twMerge } from 'tailwind-merge'
	import BatchLoadProgress from '../BatchLoadProgress.svelte'

	interface Props {
		logs?: AuditLog[]
		pageIndex?: number | undefined
		perPage?: number | undefined
		hasMore?: boolean
		actionKind?: string | undefined
		operation?: string | undefined
		selectedId?: number | undefined
		usernameFilter?: string | undefined
		resourceFilter?: string | undefined
		showWorkspace?: boolean
		loading?: boolean
		batchProgress?: { loaded: number; total: number } | null
		batchSize?: number | null
		onBatchSizeChange?: (batchSize: number) => void
		onStopLoading?: () => void
		onselect?: (id: number) => void
	}

	let {
		logs = [],
		pageIndex = $bindable(1),
		perPage = $bindable(100),
		hasMore = true,
		actionKind = $bindable(),
		operation = $bindable(),
		selectedId = undefined,
		usernameFilter = $bindable(),
		resourceFilter = $bindable(),
		showWorkspace = false,
		batchProgress = null,
		batchSize = null,
		onBatchSizeChange,
		onStopLoading,
		onselect,
		loading
	}: Props = $props()

	function groupLogsByDay(logs: AuditLog[]): Record<string, AuditLog[]> {
		const groupedLogs = {}

		if (!logs) return groupedLogs

		for (const log of logs) {
			const date = new Date(log.timestamp)
			const key = date.toLocaleString('en-US', {
				day: 'numeric',
				month: 'long',
				year: 'numeric'
			})

			if (!groupedLogs[key]) {
				groupedLogs[key] = []
			}

			groupedLogs[key].push(log)
		}
		return groupedLogs
	}

	type FlatLogs =
		| {
				type: 'date'
				date: string
		  }
		| {
				type: 'log'
				log: AuditLog
		  }

	function flattenLogs(groupedLogs: Record<string, AuditLog[]>): Array<FlatLogs> {
		const flatLogs: Array<FlatLogs> = []

		for (const [date, logsByDay] of Object.entries(groupedLogs)) {
			flatLogs.push({ type: 'date', date })
			for (const log of logsByDay) {
				flatLogs.push({ type: 'log', log })
			}
		}

		return flatLogs
	}

	let tableHeight: number = $state(0)
	let headerHeight: number = $state(0)
	let footerHeight: number = $state(48)

	function computeHeight() {
		tableHeight =
			document.querySelector('#audit-logs-table-wrapper')!.parentElement?.clientHeight ?? 0
	}

	onMount(() => {
		tick().then(computeHeight)
	})

	let groupedLogs = $derived(groupLogsByDay(logs))
	let flatLogs = $derived(groupedLogs ? flattenLogs(groupedLogs) : undefined)
	let stickyIndices = $derived.by(() => {
		const nstickyIndices: number[] = []
		let index = 0
		for (const entry of flatLogs ?? []) {
			if (entry.type === 'date') {
				nstickyIndices.push(index)
			}
			index++
		}
		return nstickyIndices
	})

	function kindToBadgeColor(kind: string) {
		if (kind == 'Execute') {
			return 'blue'
		} else if (kind == 'Delete') {
			return 'red'
		} else if (kind == 'Update') {
			return 'yellow'
		} else if (kind == 'Create') {
			return 'green'
		}
		return 'gray'
	}
	let height = $derived(tableHeight - headerHeight - footerHeight)
</script>

<svelte:window onresize={() => computeHeight()} />

<div class="divide-y min-w-[640px] h-full" id="audit-logs-table-wrapper">
	<div bind:clientHeight={headerHeight}>
		<div
			class="flex flex-row bg-surface-secondary sticky top-0 w-full p-2 pr-4 text-2xs font-normal text-primary"
		>
			<div class="w-1/12">ID</div>
			<div class={showWorkspace ? 'w-2/12' : 'w-3/12'}>Timestamp</div>
			<div class={showWorkspace ? 'w-2/12' : 'w-3/12'}>Username</div>
			{#if showWorkspace}
				<div class="w-2/12">Workspace</div>
			{/if}
			<div class={showWorkspace ? 'w-2/12' : 'w-3/12'}>Operation</div>
			<div class="w-2/12">Resource</div>
		</div>
	</div>

	{#if loading}
		<div style="height: {height}px;" class="flex justify-center items-center">
			<Loader2 class="animate-spin" />
		</div>
	{:else if !logs?.length}
		<div
			class="text-xs text-secondary p-8 flex justify-center items-center"
			style="height: {height}px;"
		>
			No logs found for the selected filters.
		</div>
	{:else}
		<VirtualList
			width="100%"
			{height}
			itemCount={flatLogs?.length ?? 0}
			itemSize={(index) => {
				if (flatLogs?.[index]?.type === 'date') {
					return 33
				}
				return 42
			}}
			overscanCount={20}
			{stickyIndices}
			scrollToAlignment="center"
		>
			{#snippet header()}{/snippet}
			{#snippet item({ index, style })}
				<div {style} class="w-full">
					{#if flatLogs}
						{@const logOrDate = flatLogs[index]}

						{#if logOrDate}
							{#if logOrDate?.type === 'date'}
								<div
									class="bg-surface-secondary py-2 border-b font-normal text-primary text-xs pl-5"
								>
									{logOrDate.date}
								</div>
							{:else}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class={twMerge(
										'flex flex-row items-center h-full w-full px-2 py-1 hover:bg-surface-hover cursor-pointer',
										'text-primary text-xs',
										logOrDate.log.id === selectedId ? 'bg-blue-50 dark:bg-blue-900/50' : ''
									)}
									role="button"
									tabindex="0"
									onclick={() => {
										onselect?.(logOrDate.log.id)
									}}
								>
									<div class="w-1/12 text-xs truncate">
										{logOrDate.log.id}
									</div>
									<div class={showWorkspace ? 'w-2/12 text-xs' : 'w-3/12 text-xs'}>
										{displayDate(logOrDate.log.timestamp)}
									</div>
									<div class={showWorkspace ? 'w-2/12 text-xs' : 'w-3/12 text-xs'}>
										<div class="flex flex-row gap-2 items-center">
											<!-- end_user can be an arbitrarily long token label; truncate it rather
											than let it push the username out of the cell. -->
											<div class="flex flex-row min-w-0 max-w-60 overflow-hidden">
												<span class="whitespace-nowrap shrink-0" title={logOrDate.log.username}>
													{logOrDate.log.username}
												</span>
												{#if logOrDate.log.parameters && 'end_user' in logOrDate.log.parameters}
													<span
														class="truncate pl-1"
														title={String(logOrDate.log.parameters.end_user)}
													>
														({logOrDate.log.parameters.end_user})
													</span>
												{/if}
											</div>
											<Button
												variant="subtle"
												unifiedSize="sm"
												iconOnly
												startIcon={{ icon: ListFilterPlus }}
												on:click={() => {
													usernameFilter = logOrDate.log.username
												}}
											/>
										</div>
									</div>
									{#if showWorkspace}
										<div class="w-2/12 text-xs">
											<div class="whitespace-nowrap overflow-x-auto no-scrollbar max-w-60">
												{logOrDate.log.workspace_id}
											</div>
										</div>
									{/if}
									<div class={showWorkspace ? 'w-2/12 text-xs' : 'w-3/12 text-xs'}>
										<div class="flex flex-row gap-1">
											<Badge
												clickable
												onclick={() => {
													actionKind = logOrDate.log.action_kind.toLocaleLowerCase()
												}}
												color={kindToBadgeColor(logOrDate.log.action_kind)}
											>
												{logOrDate.log.action_kind}
											</Badge>
											<Badge
												clickable
												onclick={() => {
													operation = logOrDate.log.operation
												}}
											>
												{logOrDate.log.operation}
											</Badge>
										</div>
									</div>
									<div class="w-2/12 text-xs">
										<div class="flex flex-row gap-2 items-center">
											<div class="whitespace-nowrap overflow-x-auto no-scrollbar max-w-60">
												{logOrDate.log.resource}
											</div>
											<Button
												variant="subtle"
												unifiedSize="sm"
												iconOnly
												startIcon={{ icon: ListFilterPlus }}
												on:click={() => {
													resourceFilter = logOrDate.log.resource
												}}
											/>
										</div>
									</div>
								</div>
							{/if}
						{:else}
							<div class="flex flex-row items-center h-full w-full px-2">
								<div class="text-xs text-secondary">Loading...</div>
							</div>
						{/if}
					{:else}
						<div class="flex flex-row items-center h-full w-full px-2">
							<div class="text-xs text-secondary">Loading...</div>
						</div>
					{/if}
				</div>
			{/snippet}
			{#snippet footer()}{/snippet}
		</VirtualList>
	{/if}
	<!-- Pagination footer - always visible -->
	<div
		class="flex flex-row justify-between items-center p-2 bg-surface-primary border-t"
		style:height={`${footerHeight}px`}
	>
		<div class="flex flex-row gap-2 items-center">
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: ChevronLeft }}
				on:click={() => {
					pageIndex = (pageIndex ?? 1) - 1
				}}
				disabled={pageIndex <= 1}
			>
				Previous
			</Button>
			<span class="text-xs text-secondary px-2">Page {pageIndex}</span>
			<Button
				variant="subtle"
				unifiedSize="sm"
				endIcon={{ icon: ChevronRight }}
				on:click={() => {
					pageIndex = (pageIndex ?? 1) + 1
				}}
				disabled={!hasMore}
			>
				Next
			</Button>
		</div>
		{#if batchProgress}
			<div class="flex-1 min-w-0 px-4">
				<BatchLoadProgress
					loaded={batchProgress.loaded}
					total={batchProgress.total}
					itemsLabel="logs"
					{batchSize}
					onBatchSizeChange={(size) => onBatchSizeChange?.(size)}
					onStop={() => onStopLoading?.()}
				/>
			</div>
		{/if}
		<div class="flex flex-row gap-2 items-center">
			<span class="text-xs text-secondary">Per page:</span>
			<select bind:value={perPage} class="text-xs border rounded-md px-2 py-1">
				<option value={25}>25</option>
				<option value={100}>100</option>
				<option value={1000}>1000</option>
			</select>
		</div>
	</div>
</div>

<style lang="postcss">
	/* Hide scrollbar for Chrome, Safari and Opera */
	.no-scrollbar::-webkit-scrollbar {
		display: none;
	}

	/* Hide scrollbar for IE, Edge and Firefox */
	.no-scrollbar {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}

	/* VirtualList scrollbar styling */
	:global(.virtual-list-wrapper:hover::-webkit-scrollbar) {
		width: 8px !important;
		height: 8px !important;
	}
</style>
