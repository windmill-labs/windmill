<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { Button } from '$lib/components/common'
	import { Bot, ChevronRight, Code2, Loader2, Plus } from 'lucide-svelte'
	import { overlayHostActive, topmostSurface } from '$lib/components/common/overlayHost.svelte'
	import type { EvalDataset, EvalExperiment, ExperimentScore } from '$lib/gen'
	import { datasetSummary, experimentName, formatScore, subjectLabel } from './evalUtils'

	let {
		experiments,
		datasets,
		caseProgress,
		loaded,
		active = false,
		deployedHash = undefined,
		currentVersion = undefined,
		onOpen,
		onHighlight,
		onEditDataset,
		onNew
	}: {
		/** Every run of this agent, newest first, whichever dataset each is of. */
		experiments: EvalExperiment[]
		/** Whether the list has been read: an empty table is a statement about the agent. */
		loaded: boolean
		/** Whether this list is the page on screen. The keyboard is only answered while it is: the
		 *  run page keeps its own rows, and both would otherwise move on one press. */
		active?: boolean
		/** The workspace's datasets, for naming the one a run is of by what it is for. */
		datasets: EvalDataset[]
		/** How many cases each still-running run has finished, keyed by run id. A run the flow has
		 *  not been read for yet is at none of them rather than absent: the count is on the row from
		 *  the moment it appears, so it never arrives late and shifts the column. */
		caseProgress: Record<string, number>
		/** What the agent hashes to as deployed, and the version it is on: they resolve a run of
		 *  edits that were later saved, so a run is labelled here as the run picker labels it. */
		deployedHash?: string
		currentVersion?: number
		onOpen: (experiment: EvalExperiment) => void
		/** The highlighted run, reported up so the surface can act on it — arrowing into the run
		 *  page opens the run under the highlight rather than whichever was opened last. */
		onHighlight?: (id: string | undefined) => void
		onEditDataset: (dataset: string) => void
		onNew: () => void
	} = $props()

	/** The highlighted run, by id. One state for both the pointer and the keyboard, as a melt menu
	 *  does it: hovering a row moves the highlight to it, so the arrows carry on from wherever the
	 *  pointer left off instead of running a second, invisible cursor of their own. It says where
	 *  the highlight is, not what is chosen — a run is not opened until Enter.
	 *
	 *  By id and not by index: the list is newest-first and the poll prepends to it, so an index
	 *  would quietly come to mean a different run and Enter would open the wrong one. */
	let cursorId = $state<string | undefined>(undefined)
	let cursor = $derived(
		cursorId === undefined ? -1 : experiments.findIndex((e) => e.id === cursorId)
	)
	let body: HTMLTableSectionElement | undefined = $state()

	// A window listener answers keys aimed anywhere, so it has to ask two questions the DOM cannot:
	// is my host the visible one — session preview tabs stay mounted when hidden — and is my surface
	// still the one on top, rather than under a drawer or a dialog opened since.
	const hostActive = overlayHostActive()
	const onTop = topmostSurface()
	const listening = () => hostActive() && onTop()

	$effect(() => {
		onHighlight?.(cursorId)
	})

	// A highlight on a run that has since gone, and the highlight itself when the list is not the
	// page on screen.
	$effect(() => {
		if (!active || (cursorId !== undefined && cursor < 0)) cursorId = undefined
	})

	function move(by: number) {
		if (experiments.length === 0) return
		const from = cursor < 0 ? (by > 0 ? -1 : experiments.length) : cursor
		const at = Math.max(0, Math.min(experiments.length - 1, from + by))
		cursorId = experiments[at]?.id
		// `nearest`, so arrowing through a long list scrolls by a row rather than jumping the table.
		requestAnimationFrame(() =>
			body?.querySelectorAll('tr')[at]?.scrollIntoView({ block: 'nearest' })
		)
	}

	function onKeydown(event: KeyboardEvent) {
		if (!active || !listening() || event.metaKey || event.ctrlKey || event.altKey) return
		const el = event.target as HTMLElement | null
		if (el?.closest?.('input, textarea, select, [contenteditable="true"], [role="listbox"]')) return
		if (event.key === 'ArrowDown') {
			event.preventDefault()
			move(1)
		} else if (event.key === 'ArrowUp') {
			event.preventDefault()
			move(-1)
		} else if (event.key === 'Enter' && experiments[cursor]) {
			// Enter belongs to whatever is focused if that thing does something with it. A highlighted
			// row is not a reason to swallow the press on `New evaluation` or a row's dataset button.
			if (el?.closest?.('button, a[href], [role="button"], summary')) return
			event.preventDefault()
			onOpen(experiments[cursor])
		}
	}

	/** The one number a column reports: a pass rate where it has a line to pass, the mean where it
	 *  does not. */
	function headline(score: ExperimentScore): string | undefined {
		if (score.pass_rate != undefined) return `${Math.round(score.pass_rate * 100)}%`
		return score.mean == undefined ? undefined : formatScore(score.mean)
	}
</script>

<svelte:window onkeydown={onKeydown} />

<DataTable size="sm" tableFixed>
	<colgroup>
		<col style="width: 20%" />
		<col style="width: 22%" />
		<col style="width: 6rem" />
		<col />
		<col style="width: 7rem" />
		<col style="width: 2rem" />
	</colgroup>
	<Head>
		<tr>
			<Cell head first>Run</Cell>
			<Cell head>Dataset</Cell>
			<Cell head numeric>Cases</Cell>
			<Cell head>Scores</Cell>
			<Cell head numeric>When</Cell>
			<Cell head last></Cell>
		</tr>
	</Head>
	<tbody class="divide-y" bind:this={body}>
		{#each experiments as experiment, i (experiment.id)}
			<!-- No `hoverable`, and not `selected` either: the first is a second hover tint competing
			     with this highlight, the second is the blue that means "chosen". Nothing here is
			     chosen until Enter, so the highlight wears the ordinary hover surface. -->
			<Row
				class="cursor-pointer {i === cursor ? 'bg-surface-hover' : ''}"
				on:click={() => onOpen(experiment)}
				on:hover={(e) => e.detail && (cursorId = experiment.id)}
			>
				<Cell first>
					<div class="flex flex-col min-w-0">
						<div class="flex items-center gap-1.5 min-w-0">
							<span class="truncate text-emphasis font-medium">{experimentName(experiment)}</span>
							<Badge color="gray" class="shrink-0">
								{subjectLabel(experiment, deployedHash, currentVersion)}
							</Badge>
						</div>
						<span class="text-2xs text-tertiary truncate">{experiment.created_by}</span>
					</div>
				</Cell>
				<Cell>
					{@const summary = datasetSummary(datasets, experiment.dataset)}
					<Button
						variant="subtle"
						unifiedSize="sm"
						title={`Edit ${experiment.dataset}`}
						wrapperClasses="min-w-0 max-w-full"
						btnClasses="!h-auto py-1 !px-0 !font-normal flex-col items-start text-left hover:underline hover:!bg-transparent min-w-0 max-w-full"
						onClick={(e) => {
							e?.stopPropagation()
							onEditDataset(experiment.dataset)
						}}
					>
						<span class="text-xs text-secondary truncate leading-tight">
							{summary || experiment.dataset}
						</span>
						{#if summary}
							<span class="text-2xs text-tertiary truncate leading-tight">
								{experiment.dataset}
							</span>
						{/if}
					</Button>
				</Cell>
				<Cell numeric>
					{#if experiment.running}
						<!-- The run's only spinner. One per score badge instead read as the judge being
						     slow, when what is unfinished is the run. -->
						<span
							class="inline-flex items-center gap-1 tabular-nums text-secondary whitespace-nowrap"
						>
							<Loader2 size={11} class="animate-spin text-blue-500 shrink-0" />
							{caseProgress[experiment.id] ?? 0}/{experiment.case_count}
						</span>
					{:else}
						<span class="tabular-nums text-secondary">{experiment.case_count}</span>
					{/if}
				</Cell>
				<Cell>
					<div class="flex flex-wrap gap-1 min-w-0">
						{#each experiment.scores ?? [] as score (score.scorer_id)}
							{@const value = headline(score)}
							<Badge color="gray" class="max-w-full">
								<span class="flex items-baseline gap-1 min-w-0">
									{#if score.kind === 'agent'}
										<Bot size={11} class="text-tertiary shrink-0 self-center" />
									{:else}
										<Code2 size={11} class="text-tertiary shrink-0 self-center" />
									{/if}
									<span class="truncate">{score.name}</span>
									{#if value != undefined}
										<span class="tabular-nums font-semibold text-emphasis">{value}</span>
									{:else if score.failed > 0}
										<span class="text-red-500">failed</span>
									{:else}
										<span class="text-tertiary">—</span>
									{/if}
								</span>
							</Badge>
						{/each}
						{#if (experiment.scores ?? []).length === 0}
							<!-- Not a pending state: a scorer's column is on the run from the moment it is
							     created, with or without a number in it, so nothing arrives here later. -->
							<span class="text-2xs text-tertiary">not scored</span>
						{/if}
					</div>
				</Cell>
				<Cell numeric>
					<span class="text-2xs text-tertiary whitespace-nowrap">
						<TimeAgo date={experiment.created_at} agoOnlyIfRecent />
					</span>
				</Cell>
				<Cell last numeric>
					<ChevronRight
						size={14}
						class="text-tertiary transition-opacity {i === cursor ? 'opacity-100' : 'opacity-0'}"
					/>
				</Cell>
			</Row>
		{/each}
		{#if experiments.length === 0 && !loaded}
			<tr>
				<td colspan="6" class="p-3">
					<Skeleton layout={[[2], 0.5, [2], 0.5, [2]]} />
				</td>
			</tr>
		{:else if experiments.length === 0}
			<tr>
				<td colspan="6" class="p-6">
					<div class="flex flex-col items-center justify-center gap-3 text-center">
						<span class="text-sm text-emphasis">No runs yet</span>
						<span class="text-xs text-secondary max-w-md">
							A run answers every case of a dataset and scores the answers. Each one is kept, so the
							next has something to be compared against.
						</span>
						<Button unifiedSize="md" variant="accent" startIcon={{ icon: Plus }} onclick={onNew}>
							New evaluation
						</Button>
					</div>
				</td>
			</tr>
		{/if}
	</tbody>
</DataTable>
