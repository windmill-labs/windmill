<script lang="ts">
	import {
		InputService,
		type Input,
		type Job,
		type RunnableType,
		type RunStartedBy
	} from '$lib/gen'
	import { createEventDispatcher, onDestroy, untrack } from 'svelte'
	import { ChevronDown, ChevronRight, ExternalLink, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import InfiniteList from './InfiniteList.svelte'
	import JobSchemaPicker from './schema/JobSchemaPicker.svelte'
	import { Cell, Row } from './table'
	import { Button } from './common'
	import { formatDate, formatDateShort, getJobKindIcon } from '$lib/utils'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import {
		innerRunLabel,
		innerRunsLabel,
		isDefaultSelection,
		isGroupEntry,
		lastSegment,
		logRunHistoryGroupExpand,
		shortId,
		shortPath,
		startedByIcon,
		topRunIcon
	} from './runHistory'

	const operatingWorkspace = useOperatingWorkspace()
	const dispatch = createEventDispatcher()

	interface Props {
		runnableId: string | undefined
		runnableType: RunnableType | undefined
		selected?: string | undefined
		showAuthor?: boolean
		placement?: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end'
		limitPayloadSize?: boolean
		searchArgs?: Record<string, any> | undefined
		/** Kinds of runs to list, runs inside other runs grouped by the top-level run that holds
		 * them. Undefined lists only completed top-level runs. */
		startedBy?: RunStartedBy[] | undefined
		/** Workspace to read run history from; defaults to the operating workspace. */
		workspace?: string
	}

	let {
		runnableId,
		runnableType,
		selected = undefined,
		showAuthor = false,
		placement = 'bottom-start',
		limitPayloadSize = false,
		searchArgs = undefined,
		startedBy = undefined,
		workspace = undefined
	}: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)
	// An archived script version is read by hash, which the categorized history does not cover.
	let categorized = $derived(startedBy != undefined && runnableType !== 'ScriptHash')

	let infiniteList: InfiniteList | undefined = $state(undefined)
	let loadInputsPageFn: ((page: number, perPage: number) => Promise<any>) | undefined = undefined
	let viewerOpen = $state(false)
	let openStates: Record<string, boolean> = {} // Track open state for each item
	let listLoading = $state(false)

	type OpenGroup = { loading: boolean; runs: any[]; summary: string }
	let openGroups: Record<string, OpenGroup> = $state({})
	let hoveredInner: string | undefined = $state(undefined)

	export function loading(): boolean {
		return listLoading
	}

	export async function refresh(clearCurrentRuns: boolean = false) {
		if (clearCurrentRuns) {
			infiniteList?.reset()
			openGroups = {}
		}
		if (infiniteList) {
			await infiniteList.loadData('refresh')
		}
	}
	let cachedArgs: Record<string, any> = {}

	let timeout: number | undefined = undefined
	function refreshInterval() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(async () => {
			await refresh()
			refreshInterval()
		}, 10000)
	}

	/** What a refresh compares to tell that an entry changed, a running run finishing included. */
	function summary(entry: Input): string {
		const g = entry.group
		const top = entry.top_run
		return [
			entry.created_at,
			entry.running,
			entry.suspended,
			entry.success,
			top?.running,
			top?.success,
			g?.runs,
			g?.failed,
			g?.running
		].join()
	}

	async function withPayload(input: Input) {
		if (isGroupEntry(input)) {
			return input
		}
		const isPreview = !!input.is_preview
		let payloadData: any
		if (input.running && input.args != undefined) {
			// A run in progress comes with its args, since a preprocessor may still replace them.
			payloadData = filterPreviewArgs(input.args, isPreview)
		} else if (cachedArgs[input.id]) {
			return { ...input, payloadData: cachedArgs[input.id] }
		} else {
			payloadData = await loadArgsFromHistory(input.id, undefined, false, isPreview)
		}
		if (payloadData === 'WINDMILL_TOO_BIG') {
			return {
				...input,
				payloadData: 'WINDMILL_TOO_BIG',
				getFullPayload: () => loadArgsFromHistory(input.id, undefined, true, isPreview)
			}
		}
		if (!input.running) {
			cachedArgs[input.id] = payloadData
		}
		return { ...input, payloadData }
	}

	/** A refresh replaces a row when this changes: its status, or the args of a run in progress. */
	function rowKey(item: any): string {
		return `${item.id}:${summary(item)}:${item.running ? JSON.stringify(item.payloadData) : ''}`
	}

	function initLoadInputs() {
		refreshInterval()
		loadInputsPageFn = async (page: number, perPage: number) => {
			if (categorized && startedBy?.length === 0) {
				return []
			}
			const inputs = (await InputService.getInputHistory({
				workspace: ws!,
				runnableId,
				runnableType,
				page,
				perPage,
				includePreview: true,
				startedBy: categorized ? startedBy?.join(',') : undefined,
				args: searchArgs ? JSON.stringify(searchArgs) : undefined
			})) as Input[]

			const inputsWithPayload = await Promise.all(inputs.map(withPayload))
			for (const entry of inputs) {
				const open = openGroups[entry.id]
				// A run in progress may still have its args replaced, without the summary changing.
				const stale = open?.summary !== summary(entry) || open?.runs.some((run) => run.running)
				if (open && !open.loading && stale) {
					loadGroup(entry)
				}
			}
			return inputsWithPayload
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	async function loadGroup(entry: Input) {
		openGroups[entry.id] = {
			loading: true,
			runs: openGroups[entry.id]?.runs ?? [],
			summary: summary(entry)
		}
		try {
			const runs = (await InputService.getInputHistory({
				workspace: ws!,
				runnableId,
				runnableType,
				startedBy: startedBy?.join(','),
				topRun: entry.top_run?.id ?? entry.id,
				args: searchArgs ? JSON.stringify(searchArgs) : undefined
			})) as Input[]
			if (!openGroups[entry.id]) {
				return
			}
			openGroups[entry.id] = {
				loading: false,
				summary: summary(entry),
				runs: runs.map((run) => {
					if (run.args === 'WINDMILL_TOO_BIG') {
						return {
							...run,
							payloadData: 'WINDMILL_TOO_BIG',
							getFullPayload: () => loadArgsFromHistory(run.id, undefined, true)
						}
					}
					if (!run.running) {
						cachedArgs[run.id] = run.args
					}
					return { ...run, payloadData: run.args }
				})
			}
		} catch (err) {
			delete openGroups[entry.id]
			dispatch('error', { type: 'load', error: err })
		}
	}

	function toggleGroup(entry: Input) {
		if (openGroups[entry.id]) {
			delete openGroups[entry.id]
		} else {
			loadGroup(entry)
			logRunHistoryGroupExpand(isGroupEntry(entry) ? 'group' : 'run')
		}
	}

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})

	function filterPreviewArgs(args: any, isPreview: boolean): any {
		if (!isPreview || !args || typeof args !== 'object' || Array.isArray(args)) return args
		const { _MODULES, ...rest } = args
		return rest
	}

	async function loadArgsFromHistory(
		id: string | undefined,
		input: boolean | undefined,
		allowLarge: boolean,
		isPreview: boolean = false
	): Promise<any> {
		if (!id) return
		const payloadData = await InputService.getArgsFromHistoryOrSavedInput({
			jobOrInputId: id,
			workspace: ws!,
			input,
			allowLarge
		})
		return filterPreviewArgs(payloadData, isPreview)
	}

	function updateViewerOpenState(itemId: string, isOpen: boolean) {
		openStates[itemId] = isOpen
		viewerOpen = Object.values(openStates).some((state) => state)
	}

	function onSelect(item: any) {
		if (isGroupEntry(item)) {
			toggleGroup(item)
		} else {
			dispatch('select', item)
		}
	}

	$effect(() => {
		ws && runnableId && runnableType && infiniteList && untrack(() => initLoadInputs())
	})
</script>

<InfiniteList
	bind:this={infiniteList}
	bind:loading={listLoading}
	selectedItemId={selected}
	itemKey={rowKey}
	on:error
	on:select={(e) => onSelect(e.detail)}
>
	{#snippet columns()}
		<colgroup>
			<col class="w-8" />
			<col class="w-16" />
			<col />
		</colgroup>
	{/snippet}
	{#snippet children({ item, hover })}
		{#if isGroupEntry(item)}
			{@const top = item.top_run}
			{@const KindIcon = getJobKindIcon(top.kind as Job['job_kind'])}
			{@const origin = topRunIcon(top)}
			<Cell>
				<div class="center-center text-tertiary">
					{#if openGroups[item.id]}
						<ChevronDown size={14} />
					{:else}
						<ChevronRight size={14} />
					{/if}
				</div>
			</Cell>
			<Cell wrap class="text-2xs font-normal text-left py-2" title={formatDate(item.created_at)}>
				{formatDateShort(item.created_at)}
			</Cell>
			<Cell class="items-center flex flex-row gap-2">
				<div class="flex items-center gap-1.5 grow min-w-0 text-xs">
					{#if top.running}
						<div class="rounded-full w-2 h-2 shrink-0 animate-pulse bg-orange-400" title="Running"
						></div>
					{:else if top.success != undefined}
						<div
							class="rounded-full w-2 h-2 shrink-0 {top.success ? 'bg-green-400' : 'bg-red-400'}"
							title={top.success ? 'Success' : 'Failed'}
						></div>
					{/if}
					{#if KindIcon}
						<KindIcon size={14} class="shrink-0 text-tertiary" />
					{/if}
					<span class="truncate text-primary" title={top.path ?? top.id}>
						{top.path ? shortPath(top.path) : 'Run'}
					</span>
					<span class="shrink-0 text-tertiary" title={top.id}>·{shortId(top.id)}</span>
					{#if origin}
						<span class="center-center shrink-0 text-tertiary" title={origin.title}>
							<origin.icon size={14} />
						</span>
					{/if}
				</div>
				<span class="shrink-0 text-2xs text-secondary whitespace-nowrap">
					{item.group.runs}{item.group.complete ? '' : '+'}
					{item.group.runs === 1 ? 'run' : 'runs'}
					{#if item.group.failed > 0}
						· <span class="text-red-500">{item.group.failed} failed</span>
					{/if}
					{#if item.group.running > 0}
						· {item.group.running} running
					{/if}
				</span>
				<a
					target="_blank"
					href="/run/{top.id}?workspace={ws}"
					class="text-secondary px-2"
					title="See the top-level run in a new tab"
					onclick={(e) => e.stopPropagation()}
				>
					<ExternalLink size={16} />
				</a>
			</Cell>
		{:else}
			{@const inner = innerRunsLabel(item)}
			{#snippet innerRunsButton()}
				<Button
					variant="subtle"
					unifiedSize="xs"
					endIcon={{ icon: openGroups[item.id] ? ChevronDown : ChevronRight }}
					onClick={() => toggleGroup(item)}
				>
					{inner}
				</Button>
			{/snippet}
			{#snippet insideRun()}
				<span
					class="flex min-w-0 flex-col px-2 leading-tight"
					title="Inside {item.top_run.path ?? 'a run'} ({item.top_run.id})"
				>
					<span class="truncate text-secondary">
						{item.top_run.path ? lastSegment(item.top_run.path) : 'Run'}
					</span>
					<span class="text-2xs text-tertiary">·{shortId(item.top_run.id)}</span>
				</span>
			{/snippet}
			<JobSchemaPicker
				job={item}
				hovering={hover}
				payloadData={item.payloadData}
				isPreview={!!item.is_preview}
				{showAuthor}
				{placement}
				{viewerOpen}
				icon={startedByIcon(item)}
				aside={inner ? innerRunsButton : item.top_run ? insideRun : undefined}
				on:openChange={({ detail }) => {
					updateViewerOpenState(item.id, detail)
				}}
				{limitPayloadSize}
			/>
		{/if}
	{/snippet}
	{#snippet afterRow({ item })}
		{@const group = openGroups[item.id]}
		{#if group}
			{#each group.runs as run (run.id)}
				<Row
					on:click={() => dispatch('select', run)}
					on:hover={(e) => (hoveredInner = e.detail ? run.id : undefined)}
					class={twMerge(
						selected === run.id ? 'bg-surface-selected' : 'hover:bg-surface-hover',
						'cursor-pointer rounded-md'
					)}
				>
					<JobSchemaPicker
						job={run}
						hovering={hoveredInner === run.id}
						payloadData={run.payloadData}
						{placement}
						{viewerOpen}
						nested
						label={innerRunLabel(run)}
						on:openChange={({ detail }) => {
							updateViewerOpenState(run.id, detail)
						}}
						{limitPayloadSize}
					/>
				</Row>
			{/each}
			{#if group.loading && group.runs.length === 0}
				<tr>
					<td colspan="3" class="py-1.5">
						<div class="flex justify-center text-tertiary">
							<Loader2 size={14} class="animate-spin" />
						</div>
					</td>
				</tr>
			{:else if !group.loading && group.runs.length === 0}
				<tr>
					<td colspan="3" class="py-1.5 pl-10 text-2xs text-secondary">
						No runs of the selected kinds in this run
					</td>
				</tr>
			{:else if group.runs.length >= 500}
				<tr>
					<td colspan="3" class="py-1.5 pl-10 text-2xs text-secondary">
						Showing the first 500 runs.
						<a target="_blank" href="/run/{item.top_run?.id ?? item.id}?workspace={ws}">
							Open the run
						</a>
						to see the rest.
					</td>
				</tr>
			{/if}
		{/if}
	{/snippet}
	{#snippet empty()}
		<div class="text-center text-primary text-xs py-2">
			{#if !runnableId}
				Save draft to see previous runs
			{:else if categorized && startedBy && !isDefaultSelection(startedBy)}
				No runs of the selected kinds in the latest 5,000 runs
			{:else}
				No previous inputs
			{/if}
		</div>
	{/snippet}
</InfiniteList>
