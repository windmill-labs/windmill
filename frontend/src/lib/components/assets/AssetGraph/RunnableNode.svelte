<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import {
		Code2,
		EllipsisVertical,
		GitBranch,
		Layers,
		Loader2,
		Play,
		Timer,
		Trash2,
		Zap
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import type { GraphUsageKind } from './types'
	import { NODE } from '$lib/components/graph/util'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
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
			// Called before running so the details pane focuses this script —
			// mirrors AssetNode.onSelectAsset, keeps the runs/output in view
			// instead of dispatching into nowhere.
			onSelectSelf?: () => void
			// Wired by the canvas. When set, the node renders an
			// EllipsisVertical hover-button that opens a small action menu —
			// "Discard" for drafts, "Delete…" (which the page maps to its
			// archive/delete confirmation flow) for persisted scripts.
			onRequestRemove?: () => void
		}
		// SvelteFlow injects this when the user clicks the node. Combined with
		// hover state to drive the run-button visibility (same pattern as
		// AssetNode).
		selected?: boolean
	}
	let { data, selected = false }: Props = $props()

	// Icon + emerald accent already convey "pipeline script" vs "flow"; the
	// uppercase kind label was visually noisy and redundant. Tooltip on hover
	// surfaces the path in full when truncated.
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
	let canRun = $derived(data.runnable_kind === 'script' && data.onRunSelf != undefined)
	// Reveal pattern matches AssetNode: visible while hovering, selected, or
	// already running (so the loader doesn't disappear under the cursor).
	let showRun = $derived(canRun && (hover || selected || running))

	async function runSelf(e: MouseEvent, cascade?: boolean) {
		e.stopPropagation()
		if (!$workspaceStore || running || !data.onRunSelf) return
		// Focus this runnable so the details pane opens to its editor — same
		// rationale as AssetNode.onSelectAsset before runProducers.
		if (!selected) data.onSelectSelf?.()
		running = true
		try {
			await data.onRunSelf(cascade != undefined ? { cascade } : undefined)
		} catch (err: any) {
			sendUserToast(`Failed to run: ${err.body ?? err.message}`, true)
		} finally {
			running = false
		}
	}

	let menuItems: Item[] = $derived([
		// Cascade option lives at the top of the menu — the round Run button
		// is the no-cascade default, this surfaces the alternative when there
		// are downstream subscribers to fan out to.
		...(canRun && (data.downstreamCount ?? 0) > 0
			? [
					{
						displayName: `Run + trigger ${data.downstreamCount} downstream`,
						icon: Zap,
						action: () => {
							// Fake MouseEvent-shaped arg so runSelf can stopPropagation.
							void runSelf({ stopPropagation: () => {} } as MouseEvent, true)
						}
					}
				]
			: []),
		...(data.onRequestRemove
			? [
					{
						displayName: data.unsaved ? 'Discard' : 'Delete…',
						icon: Trash2,
						type: 'delete' as const,
						action: () => data.onRequestRemove?.()
					}
				]
			: [])
	])
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
	<!--
		Three visual states that compose:
		  - persisted, non-pipeline:  solid 1px gray border, surface-tertiary fill
		  - persisted, pipeline:      solid 1px emerald border, surface-tertiary fill
		  - unsaved draft:            DASHED 2px emerald border, surface-tertiary fill.
		                              The 2px stroke is necessary for the dash gaps
		                              to be readable; 1px disappears into the fill.
		                              Same tertiary surface as persisted nodes —
		                              the dash + thicker stroke alone carry the
		                              "unsaved" cue without an attention-grabbing
		                              tinted fill.
	-->
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden border transition-colors',
			'bg-surface-tertiary border-gray-300 dark:border-gray-600 hover:border-emerald-500',
			data.in_pipeline && !data.unsaved && 'border-emerald-400/60',
			data.unsaved && 'border-2 border-dashed border-emerald-400/70 dark:border-emerald-500/70'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		title={nodeTooltip}
	>
		<Icon size={14} class="shrink-0 ml-2 mr-2 text-emerald-700 dark:text-emerald-400" />
		<span class="flex-1 min-w-0 pr-1 py-0.5 text-2xs font-mono text-emphasis truncate">
			{data.path}
		</span>
		{#if data.partition_kind}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300"
				title={`// partitioned ${data.partition_kind}`}
			>
				<Layers size={10} />
				<span class="text-3xs leading-none">{data.partition_kind}</span>
			</div>
		{/if}
		{#if data.freshness}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1 py-0.5 mr-1 rounded-sm bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300"
				title={`// freshness ${data.freshness}`}
			>
				<Timer size={10} />
				<span class="text-3xs leading-none">{data.freshness}</span>
			</div>
		{/if}
	</div>
	{#if showRun}
		<!-- Run button revealed on hover/select. Matches the placement, size,
		     and behaviour of AssetNode's run button so both nodes feel
		     consistent. Drafts are runnable too (the page handler routes to
		     runScriptPreview), so no greyed-out state. -->
		<div class="absolute -left-3 top-1/2 -translate-y-1/2 z-10">
			<button
				type="button"
				onclick={runSelf}
				disabled={running}
				class="bg-blue-500 hover:bg-blue-600 disabled:opacity-60 text-white rounded-full w-6 h-6 grid place-items-center shadow border-2 border-surface-secondary leading-none"
				title={`Run ${data.path}${data.unsaved ? ' (draft, runs as preview)' : ''}`}
			>
				{#if running}
					<Loader2 size={14} class="animate-spin" />
				{:else}
					<Play size={14} strokeWidth={2.5} class="translate-x-px" />
				{/if}
			</button>
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
