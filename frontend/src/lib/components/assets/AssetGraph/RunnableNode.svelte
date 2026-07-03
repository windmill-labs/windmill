<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import {
		CheckCircle2,
		ChevronDown,
		Code2,
		EllipsisVertical,
		GitBranch,
		Layers,
		Loader2,
		Play,
		RotateCw,
		SquareFunction,
		Tag,
		Target,
		Timer,
		Trash2,
		XCircle,
		Zap
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import type { GraphUsageKind } from './types'
	import type { RunnableRunState } from './activeRunnables.svelte'
	import { NODE } from '$lib/components/graph/util'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { Item } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'

	interface Props {
		data: {
			runnable_kind: GraphUsageKind
			path: string
			in_pipeline?: boolean
			partition_kind?: 'daily' | 'hourly' | 'weekly' | 'monthly' | 'dynamic'
			freshness?: string
			tag?: string
			retry?: { count: number; delay?: string }
			// Macros this script provides (deployed/drafted `// macros` library).
			// Non-empty renders the ƒ chip marking the node as a macro library.
			macros?: { name: string; params: string; is_table: boolean }[]
			// Last-run status + run count observed this session (from the
			// folder queue poll). Undefined until the first observed run.
			runState?: RunnableRunState
			// True for nodes synthesized from local drafts (script not yet
			// persisted). Same convention as `unsaved` on triggers/edges.
			unsaved?: boolean
			// Page-supplied dispatch that runs THIS node (saved → runScriptByPath,
			// unsaved → runScriptPreview with the locally-cached draft content).
			// Wired only for script runnables — flows are ignored upstream. When
			// undefined, the play button is hidden — matches the asset-node
			// behaviour outside editor contexts. `opts.cascade` lets the
			// dispatcher decide whether to skip the asset-trigger cascade — the
			// node's default-click sends `cascade: false` (just this step) to
			// stay consistent with the editor's Test button.
			onRunSelf?: (opts?: { cascade?: boolean }) => Promise<string | undefined>
			// Number of script subscribers that listen on assets this script
			// writes. When > 0, the hover-menu exposes a "Run + trigger N
			// downstream" alternative; the round Play button stays a single-
			// click default. Undefined / 0 hides the cascade menu item.
			downstreamCount?: number
			// How many of those subscribers are unsaved drafts. The production
			// asset-trigger dispatcher can't reach drafts, so when this (or the
			// node itself) is unsaved the cascade run is orchestrated by the
			// page instead, and the menu item says so — making the dev-run vs
			// deployed-cascade distinction explicit.
			downstreamUnsavedCount?: number
			// Wired by the canvas. When set, the node renders an
			// EllipsisVertical hover-button that opens a small action menu —
			// "Discard" for drafts, "Delete…" (which the page maps to its
			// archive/delete confirmation flow) for persisted scripts.
			onRequestRemove?: () => void
			// Wired only for valid bounded-run starts (schedule / manual roots
			// with downstream). Enters the page's end-node pick mode for a
			// bounded cascade rooted at this script.
			onStartBoundedRun?: () => void
		}
		// SvelteFlow injects this when the user clicks the node. Combined with
		// hover state to drive the run-button visibility (same pattern as
		// AssetNode).
		selected?: boolean
	}
	let { data, selected = false }: Props = $props()

	// The icon alone conveys "script" vs "flow"; the uppercase kind label was
	// visually noisy and redundant. Tooltip on hover surfaces the path in
	// full when truncated.
	let Icon = $derived(data.runnable_kind === 'flow' ? GitBranch : Code2)
	let nodeTooltip = $derived(
		data.unsaved
			? `${data.path} (unsaved draft)`
			: data.in_pipeline
				? `${data.path} (pipeline member)`
				: data.path
	)

	let hover = $state(false)
	let menuOpen = $state(false)
	let running = $state(false)
	// Popover state for the on-node Run-button caret. Sticky while open so
	// `showRun` (which gates the whole pill) stays true even after the
	// pointer leaves the node — otherwise picking an option would unmount
	// the Popover mid-click.
	let cascadeMenuOpen = $state(false)
	let canRun = $derived(data.runnable_kind === 'script' && data.onRunSelf != undefined)
	// Reveal pattern matches AssetNode: visible while hovering, selected, or
	// already running (so the loader doesn't disappear under the cursor).
	let showRun = $derived(canRun && (hover || selected || running || cascadeMenuOpen))

	async function runSelf(e: MouseEvent, cascade?: boolean) {
		e.stopPropagation()
		if (!$workspaceStore || running || !data.onRunSelf) return
		running = true
		try {
			await data.onRunSelf(cascade != undefined ? { cascade } : undefined)
		} catch (err: any) {
			sendUserToast(`Failed to run: ${err.body ?? err.message}`, true)
		} finally {
			running = false
		}
	}

	// Cascade + bounded-run options live on the Run button's caret popover
	// (whenever there's a cascade OR a bounded-run start — see `hasCaret`
	// below), so the kebab menu stays focused on lifecycle actions only.
	let menuItems: Item[] = $derived(
		data.onRequestRemove
			? [
					{
						displayName: data.unsaved ? 'Discard' : 'Delete…',
						icon: Trash2,
						type: 'delete' as const,
						action: () => data.onRequestRemove?.()
					}
				]
			: []
	)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
	<!--
		Mirrors the flow editor's step styling (getNodeColorClasses): muted
		surface-tertiary fill with a quiet gray border, accent only for the
		selected state, dashes for unsaved drafts. The 2px dashed stroke is
		necessary for the dash gaps to be readable; 1px disappears into the
		fill.
	-->
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden border transition-colors',
			'bg-surface border-gray-400 dark:border-gray-600 hover:border-gray-500 dark:hover:border-gray-500',
			selected && 'bg-surface-accent-selected border-border-selected',
			data.unsaved && 'border-2 border-dashed border-gray-400 dark:border-gray-500'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		title={nodeTooltip}
	>
		<Icon size={14} class={`shrink-0 ml-2 mr-2 ${selected ? 'text-accent' : 'text-secondary'}`} />
		<span class="flex-1 min-w-0 pr-1 py-0.5 text-2xs font-mono text-emphasis truncate">
			{data.path}
		</span>
		<!-- Annotation chips share one neutral treatment — the icon carries
		     the meaning (colors are reserved for feedback, per the brand
		     guidelines). Only the run-state chip below keeps semantic
		     colors. -->
		{#if data.partition_kind}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-surface-secondary text-secondary"
				title={`// partitioned ${data.partition_kind}`}
			>
				<Layers size={10} />
				<span class="text-3xs leading-none">{data.partition_kind}</span>
			</div>
		{/if}
		{#if data.freshness}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-surface-secondary text-secondary"
				title={`// freshness ${data.freshness}`}
			>
				<Timer size={10} />
				<span class="text-3xs leading-none">{data.freshness}</span>
			</div>
		{/if}
		{#if data.tag}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-surface-secondary text-secondary"
				title={`// tag ${data.tag}`}
			>
				<Tag size={10} />
				<span class="text-3xs leading-none">{data.tag}</span>
			</div>
		{/if}
		{#if data.retry}
			{@const r = data.retry}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-surface-secondary text-secondary"
				title={`// retry ${r.count}${r.delay ? ` ${r.delay}` : ''}`}
			>
				<RotateCw size={10} />
				<span class="text-3xs leading-none">×{r.count}</span>
			</div>
		{/if}
		{#if data.macros && data.macros.length > 0}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-surface-secondary text-secondary"
				title={`// macros — defines ${data.macros.length} macro${data.macros.length > 1 ? 's' : ''}:\n${data.macros
					.map((m) => `• ${m.name}(${m.params})${m.is_table ? ' → table' : ''}`)
					.join('\n')}`}
			>
				<SquareFunction size={10} />
				<span class="text-3xs leading-none">×{data.macros.length}</span>
			</div>
		{/if}
		{#if data.runState}
			{@const rs = data.runState}
			<div
				class={twMerge(
					'shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm',
					rs.status === 'running'
						? 'bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300'
						: rs.status === 'success'
							? 'bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300'
							: 'bg-red-100 dark:bg-red-900/40 text-red-700 dark:text-red-300'
				)}
				title={`${
					rs.status === 'running'
						? 'Running now'
						: rs.status === 'success'
							? 'Last run succeeded'
							: 'Last run failed'
				}${rs.runs > 0 ? ` — ran ${rs.runs}× this session` : ''}`}
			>
				{#if rs.status === 'running'}
					<Loader2 size={10} class="animate-spin" />
				{:else if rs.status === 'success'}
					<CheckCircle2 size={10} />
				{:else}
					<XCircle size={10} />
				{/if}
				{#if rs.runs > 0}
					<span class="text-3xs leading-none tabular-nums">×{rs.runs}</span>
				{/if}
			</div>
		{/if}
	</div>
	{#if showRun}
		<!-- Run button revealed on hover/select. Matches the placement, size,
		     and behaviour of AssetNode's run button so both nodes feel
		     consistent. Drafts are runnable too (the page handler routes to
		     runScriptPreview), so no greyed-out state.

		     When the script has downstream subscribers, the round button
		     becomes a small split pill: Play on the left (default = skip
		     cascade) + chevron on the right that pops a menu offering "Run
		     + trigger N downstream". Matches the editor Test split button
		     so the affordance is identical on both surfaces. -->
		{@const hasCascade = (data.downstreamCount ?? 0) > 0}
		<!-- The caret popover opens for either a subscriber cascade OR a
		     bounded-run start. A pure-reader-only root has no subscriber
		     downstream (`downstreamCount === 0`) but still gets `onStartBoundedRun`
		     — without this it would have no visible "Run downstream up to…". -->
		{@const hasCaret = hasCascade || !!data.onStartBoundedRun}
		<div class="absolute -left-3 top-1/2 -translate-y-1/2 z-10 flex items-stretch">
			<button
				type="button"
				onclick={(e) => runSelf(e)}
				disabled={running}
				class={twMerge(
					'bg-blue-500 hover:bg-blue-600 disabled:opacity-60 text-white grid place-items-center shadow border-2 border-surface-secondary leading-none',
					hasCaret ? 'rounded-l-full w-6 h-6 border-r-0' : 'rounded-full w-6 h-6'
				)}
				title={`Run ${data.path}${data.unsaved ? ' (draft, runs as preview)' : ''}`}
			>
				{#if running}
					<Loader2 size={14} class="animate-spin" />
				{:else}
					<Play size={14} strokeWidth={2.5} class="translate-x-px" />
				{/if}
			</button>
			{#if hasCaret}
				<Popover
					placement="bottom-start"
					bind:isOpen={cascadeMenuOpen}
					usePointerDownOutside
					contentClasses="p-0"
					class={twMerge(
						'bg-blue-500 hover:bg-blue-600 disabled:opacity-60 text-white grid place-items-center shadow border-2 border-surface-secondary rounded-r-full w-5 h-6 leading-none -ml-px',
						'!border-l-white/30'
					)}
				>
					{#snippet trigger()}
						<ChevronDown size={11} strokeWidth={2.5} />
					{/snippet}
					{#snippet content({ close })}
						<div class="w-72 py-1 text-xs">
							<button
								type="button"
								class="w-full text-left px-3 py-2 hover:bg-surface-hover flex items-start gap-2"
								onclick={(e) => {
									close()
									void runSelf(e, false)
								}}
							>
								<Play size={14} class="mt-0.5 shrink-0 text-secondary" />
								<div class="flex flex-col min-w-0">
									<span class="font-medium">Run</span>
									<span class="text-2xs text-secondary">
										Run just this step. Downstream subscribers are not triggered.
									</span>
								</div>
							</button>
							{#if hasCascade}
								<button
									type="button"
									class="w-full text-left px-3 py-2 hover:bg-surface-hover flex items-start gap-2"
									onclick={(e) => {
										close()
										void runSelf(e, true)
									}}
								>
									<Zap size={14} class="mt-0.5 shrink-0 text-secondary" />
									<div class="flex flex-col min-w-0">
										<span class="font-medium">
											Run + trigger {data.downstreamCount} downstream
										</span>
										<span class="text-2xs text-secondary">
											Let the asset-trigger cascade fan out to the {data.downstreamCount}
											subscribed script{data.downstreamCount === 1 ? '' : 's'} after this run succeeds.
										</span>
										{#if data.unsaved || (data.downstreamUnsavedCount ?? 0) > 0}
											<span class="text-2xs text-amber-700 dark:text-amber-400">
												{#if data.unsaved}
													Unsaved chain — runs as previews in dependency order.
												{:else}
													{data.downstreamUnsavedCount} unsaved — chain runs as previews in dependency
													order.
												{/if}
												Deploy to enable automatic triggering.
											</span>
										{/if}
									</div>
								</button>
							{/if}
							{#if data.onStartBoundedRun}
								<button
									type="button"
									class="w-full text-left px-3 py-2 hover:bg-surface-hover flex items-start gap-2 border-t"
									onclick={(e) => {
										e.stopPropagation()
										cascadeMenuOpen = false
										data.onStartBoundedRun?.()
									}}
								>
									<Target size={14} class="mt-0.5 shrink-0 text-secondary" />
									<div class="flex flex-col min-w-0">
										<span class="font-medium">Run downstream up to…</span>
										<span class="text-2xs text-secondary">
											Pick end node(s) on the graph, then run only the cascade between this script
											and them.
										</span>
									</div>
								</button>
							{/if}
						</div>
					{/snippet}
				</Popover>
			{/if}
		</div>
	{/if}

	{#if menuItems.length > 0}
		<!--
			Hover-revealed action menu, top-right of the node. Mirrors the
			FlowModuleSchemaItem pattern: position the trigger button just
			outside the node frame, only render it on hover (or while the
			menu is open) so the canvas isn't visually cluttered when idle.
			The pointerdown stop+preventDefault keeps xyflow from selecting
			the node when the user is reaching for the menu.
		-->
		<div class="absolute -top-2 -right-2 h-7 p-1 min-w-7" style="will-change: transform;">
			<DropdownV2
				items={menuItems}
				placement="bottom-end"
				bind:open={menuOpen}
				fixedHeight={false}
				usePointerDownOutside
			>
				{#snippet buttonReplacement()}
					<button
						class={twMerge(
							'center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary',
							hover || menuOpen ? 'block' : '!hidden',
							'shadow-md rounded-md'
						)}
						onpointerdown={stopPropagation(preventDefault(() => {}))}
						title="Actions"
					>
						<EllipsisVertical size={12} />
					</button>
				{/snippet}
			</DropdownV2>
		</div>
	{/if}
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
