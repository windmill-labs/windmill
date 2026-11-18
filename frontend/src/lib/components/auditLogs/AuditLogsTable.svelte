<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { AuditLog } from '$lib/gen'
	import { displayDate, pluralize } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import Select from '../select/Select.svelte'
	import { ListFilterPlus, Loader2 } from 'lucide-svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import { twMerge } from 'tailwind-merge'
	import BatchLoadProgress from '../BatchLoadProgress.svelte'

	interface Props {
		logs?: AuditLog[]
		perPage?: number | undefined
		hasMore?: boolean
		actionKind?: string | undefined
		operation?: string | undefined
		selectedId?: number | undefined
		usernameFilter?: string | undefined
		resourceFilter?: string | undefined
		showWorkspace?: boolean
		loading?: boolean
		loadingExtra?: boolean
		onLoadMore?: () => void
		batchProgress?: { loaded: number; total: number } | null
		batchSize?: number | null
		onBatchSizeChange?: (batchSize: number) => void
		onStopLoading?: () => void
		onselect?: (id: number) => void
	}

	let {
		logs = [],
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
		loading,
		loadingExtra = false,
		onLoadMore
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

	let height: number = $state(0)

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
</script>

<div class="flex flex-col min-w-[640px] h-full">
	<!-- h-8 is mirrored by the detail pane's top offset on the audit logs page, so both cards
	     start at the same height. -->
	<div
		class="flex flex-row items-center h-8 shrink-0 w-full px-2 pr-4 text-xs font-semibold text-emphasis"
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

	<div
		class="flex-1 min-h-0 border rounded-t-md overflow-clip bg-surface-tertiary"
		bind:clientHeight={height}
	>
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
				{#snippet footer()}
					{#if hasMore && !batchProgress}
						<div class="flex justify-center py-1">
							<Button
								variant="subtle"
								unifiedSize="sm"
								loading={loadingExtra}
								onClick={() => onLoadMore?.()}
							>
								Load next {perPage} logs
							</Button>
						</div>
					{/if}
				{/snippet}
			</VirtualList>
		{/if}
	</div>
	<div
		class="flex flex-row shrink-0 justify-between items-center h-12 px-2 py-1 bg-surface-tertiary border-x border-b rounded-b-md text-xs"
	>
		<span class="text-secondary px-2">{pluralize(logs?.length ?? 0, 'log')}</span>
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
		<Select
			class="w-28"
			bind:value={perPage}
			items={[
				{ value: 25, label: '25' },
				{ value: 100, label: '100' },
				{ value: 1000, label: '1000' }
			]}
			transformInputSelectedText={(_, v) => `${v} / page`}
		/>
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
