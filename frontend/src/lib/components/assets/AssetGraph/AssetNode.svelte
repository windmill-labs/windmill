<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatShortAssetPath, type AssetKind } from '$lib/components/assets/lib'
	import { NODE } from '$lib/components/graph/util'
	import PipelineInsertMenu, { type PipelineInsertPick } from './PipelineInsertMenu.svelte'
	import {
		ArrowUpRight,
		Code2,
		GitFork,
		History,
		Play,
		Loader2,
		Plus,
		ShieldCheck,
		ShieldAlert
	} from 'lucide-svelte'
	import type { ScriptLang } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { PIPELINE_LANGUAGES } from './pipelineLanguages'
	import type { PipelineOutputKind } from './pipelineTemplates'

	// Shape used for both the data prop and the run callback. Drafts carry
	// `content` / `language` so the page-level run handler can dispatch to
	// `runScriptPreview` instead of `runScriptByPath` (which 404s for
	// non-deployed scripts).
	export type AssetProducer = {
		kind: 'script' | 'flow'
		path: string
		unsaved?: boolean
	}

	interface Props {
		data: {
			asset_kind: AssetKind
			path: string
			// Fork workspaces: 'fork' = materialized in this fork, 'deferred' =
			// reads fall back to the parent workspace's current data.
			fork_materialization?: 'fork' | 'deferred'
			// Set on an SCD2 `<dim>_current` companion view: the base `<dim>` path
			// its producer materializes with `// materialize … history`. Drives the
			// "current view of <dim>" marker so it reads as a derived node, not an
			// unrelated table.
			derived_from?: string
			onAddScript?: (
				asset: { kind: AssetKind; path: string },
				language: ScriptLang,
				scriptPath: string,
				outputKind: PipelineOutputKind,
				aiPrompt?: string
			) => void
			pathPrefix?: string
			defaultPathSuffix?: string
			// Producer scripts/flows that write to this asset, supplied by
			// the canvas from the graph's write/rw edges. Drives the on-hover
			// "Run" button. Includes unsaved/draft producers so the button
			// shows up in fresh pipelines.
			producers?: AssetProducer[]
			// Page-supplied callback that knows how to actually run a
			// producer. Takes a producer and returns the new job's id.
			// Implementation can dispatch to `runScriptByPath` for saved
			// scripts and `runScriptPreview` for drafts (using the locally-
			// cached draft content). Without this callback, the play button
			// is hidden — runs only make sense in editor contexts.
			onRunProducer?: (producer: AssetProducer) => Promise<string | undefined>
			// True when a producer materializing this asset declares `// data_test`
			// checks — the asset's write is guarded. Drives the data-test outcome
			// badge, whose meaning differs by edition (EE rolls a failing write
			// back; CE publishes it anyway).
			dataTestGuarded?: boolean
			// True when the latest observed run of a producer materializing this
			// asset failed. Escalates the guard badge from "protected" to a
			// failed-run outcome (rolled-back on EE, published-anyway on CE).
			producerFailed?: boolean
			// Monotonic nonce bumped by the replay player when this asset's
			// producer just recomputed it. A change triggers a one-shot green
			// fade so a freshly-written table stands out as the run progresses.
			recomputePulse?: number
		}
		// SvelteFlow injects this on the node component when the user clicks
		// the node. Combined with our own `hovered` state to drive the
		// run-button visibility — same affordance as the flow editor's
		// hover/select-revealed step actions.
		selected?: boolean
	}
	let { data, selected = false }: Props = $props()

	let asset = $derived({ kind: data.asset_kind, path: data.path })
	let hovered = $state(false)
	let running = $state(false)
	let producers = $derived(data.producers ?? [])
	let scriptProducers = $derived(producers.filter((p) => p.kind === 'script'))
	let canRun = $derived(scriptProducers.length > 0 && data.onRunProducer != undefined)
	// Visible iff the user has opted into the node either by selecting it
	// (svelteflow's `selected` prop) or hovering. While running, we keep it
	// pinned so the loader doesn't disappear out from under the cursor.
	let showActions = $derived((selected || hovered || running) && canRun)

	async function runProducers(e: MouseEvent) {
		e.stopPropagation()
		if (!$workspaceStore || running || !data.onRunProducer) return
		if (scriptProducers.length === 0) return
		running = true
		const handler = data.onRunProducer
		try {
			// Fire every script producer in parallel — matches what would
			// happen if every upstream trigger fired together. The handler
			// internally dispatches to runScriptByPath / runScriptPreview
			// based on producer.unsaved. No success toast — the runs panel
			// auto-selects the new job and shows status/logs/output, so
			// the toast was redundant.
			await Promise.all(scriptProducers.map((p) => handler(p)))
		} catch (err: any) {
			sendUserToast(`Failed to run: ${err.body ?? err.message}`, true)
		} finally {
			running = false
		}
	}

	function handlePick(pick: PipelineInsertPick) {
		if (pick.kindId === 'pipeline_script' && pick.language && pick.path) {
			data.onAddScript?.(
				{ kind: data.asset_kind, path: data.path },
				pick.language as ScriptLang,
				pick.path,
				(pick.outputKind ?? 'none') as PipelineOutputKind,
				pick.aiPrompt
			)
		}
	}

	let showAdd = $derived(data.onAddScript != undefined)

	// Data-test outcome badge. Only guarded assets show it. The write's fate on a
	// failing test differs by edition — surface which one applies so a shared
	// parent/fork table name can't hide a silently-published bad version.
	let isEE = $derived(!!$enterpriseLicense)
	let showGuardBadge = $derived(data.dataTestGuarded === true)
	let guardFailed = $derived(data.producerFailed === true)
	let GuardIcon = $derived(isEE ? ShieldCheck : ShieldAlert)
	// Filled + colored when the last run failed (the actionable state); a quiet
	// ring at rest so the badge doesn't shout on every healthy guarded asset.
	let guardClass = $derived(
		guardFailed
			? isEE
				? 'bg-amber-500 text-white border-amber-600'
				: 'bg-red-500 text-white border-red-600'
			: isEE
				? 'bg-surface-secondary text-emerald-600 dark:text-emerald-400 border-emerald-500/60'
				: 'bg-surface-secondary text-amber-600 dark:text-amber-400 border-amber-500/60'
	)
	// Failed copy speaks to the edition's write policy, not the failure cause:
	// `producerFailed` is a generic job failure (could be a runtime/worker error,
	// not a data-test violation), so we don't assert "failed its data tests".
	let guardTitle = $derived(
		guardFailed
			? isEE
				? 'Last run failed — Enterprise rolls a failed materialize back, so the previous version is left live.'
				: 'Last run failed — Community Edition does not roll a failed materialize back (Enterprise does), so a failing write may be left live. Verify the table.'
			: isEE
				? 'Guarded by data tests: a failing write is rolled back, keeping the previous version live.'
				: 'Guarded by data tests, but Community Edition does not block on failure — a failing write is still published. Rollback is Enterprise-only.'
	)
</script>

<!-- onmouseenter/leave on the wrapper (not the inner card) so the run
     button — which floats outside the card — keeps the hover state alive
     when the cursor moves between the card and the button. -->
<div
	class="relative"
	onmouseenter={() => (hovered = true)}
	onmouseleave={() => (hovered = false)}
	role="presentation"
>
	{#if data.recomputePulse !== undefined}
		{#key data.recomputePulse}
			<!-- One-shot green wash when the replay player recomputes this asset.
			     Keyed on the nonce so a repeat recompute replays the animation. -->
			<div class="wm-recompute-flash pointer-events-none absolute inset-0 z-10 rounded-md"></div>
		{/key}
	{/if}
	<!-- Mirrors the flow editor's asset pill: quiet surface + gray border at
	     rest, accent reserved for the selected state. -->
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden border',
			'bg-surface-secondary border-gray-400 dark:border-gray-600 hover:border-gray-500 dark:hover:border-gray-500 transition-colors',
			selected && 'bg-surface-accent-selected border-border-selected'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		title={data.path}
	>
		<!-- Data identity carries the accent (luminance blue), pairing with
		     the blue write edges that produce these assets — scripts stay
		     neutral, so script vs data reads at a glance. -->
		<AssetGenericIcon
			assetKind={data.asset_kind}
			class={`shrink-0 ml-2 mr-2 ${selected ? 'text-accent' : 'text-blue-600 dark:text-blue-400'}`}
			size="14px"
		/>
		<span class="flex-1 min-w-0 pr-1 py-0.5 text-2xs font-mono text-emphasis truncate">
			{formatShortAssetPath(asset)}
		</span>
		<!-- Fork data-environment chip: in a fork every asset shares its parent's
		     name, so the env it resolves to must read at a glance. Labeled + tinted
		     (amber "parent" = deferred read of the parent's current table via a
		     view; emerald "fork" = the fork's own materialized copy) rather than a
		     bare icon, which was too easy to miss. The title carries the detail. -->
		{#if data.fork_materialization === 'deferred'}
			<span
				class="shrink-0 mr-1.5 flex items-center gap-0.5 rounded px-1 py-px text-3xs font-semibold uppercase tracking-wide bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300 border border-amber-300 dark:border-amber-700"
				title="Deferred to parent workspace: reads the parent's current data. Materialize it in this fork to iterate on it."
			>
				<ArrowUpRight size={10} />
				parent
			</span>
		{:else if data.fork_materialization === 'fork'}
			<span
				class="shrink-0 mr-1.5 flex items-center gap-0.5 rounded px-1 py-px text-3xs font-semibold uppercase tracking-wide bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300 border border-emerald-300 dark:border-emerald-700"
				title="Materialized in this fork: reads and writes use the fork's isolated copy."
			>
				<GitFork size={10} />
				fork
			</span>
		{/if}
		<!-- SCD2 companion marker: this node is the `<dim>_current` "latest row
		     per key" view its producer maintains alongside the base dimension.
		     Icon-only (the pill already truncates); the title names the base. -->
		{#if data.derived_from}
			<span
				class="shrink-0 mr-1.5 text-violet-600 dark:text-violet-400"
				title={`SCD2 current view of ${data.derived_from}: latest row per key, maintained by the same producer as the base dimension.`}
			>
				<History size={12} />
			</span>
		{/if}
	</div>
	{#if showGuardBadge}
		<!-- Data-test outcome badge. Floats off the TOP-RIGHT corner (opposite the
		     left-edge run button and the bottom + inserter) so it never collides
		     with the other node affordances. Shield = guarded; its fill escalates to
		     amber/red when the last run failed, encoding the edition's write policy. -->
		<div
			class={twMerge(
				'absolute -top-2 -right-2 z-10 rounded-full w-5 h-5 grid place-items-center border shadow-sm',
				guardClass
			)}
			title={guardTitle}
		>
			<GuardIcon size={12} strokeWidth={2.25} />
		</div>
	{/if}
	{#if showActions}
		<!-- Run button revealed on hover/select. Floats off the LEFT edge —
		     visually closer to the upstream producer it triggers (which lays
		     out above-and-toward-the-arrow-source in the DAG), and avoids
		     colliding with badges that may sit on the right of future asset
		     cards. Drafts are runnable too via runScriptPreview, so no
		     greyed-out state — the page-supplied callback handles the
		     dispatch. -->
		<div class="absolute -left-3 top-1/2 -translate-y-1/2 z-10">
			<button
				type="button"
				onclick={runProducers}
				disabled={running}
				class="bg-blue-500 hover:bg-blue-600 disabled:opacity-60 text-white rounded-full w-6 h-6 grid place-items-center shadow border-2 border-surface-secondary leading-none"
				title={scriptProducers.length > 1
					? `Run ${scriptProducers.length} upstream scripts`
					: `Run ${scriptProducers[0]?.path ?? ''}${scriptProducers[0]?.unsaved ? ' (draft, runs as preview)' : ''}`}
			>
				{#if running}
					<Loader2 size={14} class="animate-spin" />
				{:else}
					<!-- translate-x-px nudges the play triangle off-center to
					     compensate for its own asymmetric viewBox; without it
					     the triangle's centroid lands ~1px left of the button
					     center and reads as misaligned. -->
					<Play size={14} strokeWidth={2.5} class="translate-x-px" />
				{/if}
			</button>
		</div>
	{/if}
	{#if showAdd}
		<!-- Always-visible + below the asset for downstream pipeline-script
		     creation. Half-overlapping the bottom edge so it visually attaches
		     to the node like the flow editor's between-step inserter. -->
		<div class="absolute left-1/2 -bottom-3 -translate-x-1/2 z-10">
			<PipelineInsertMenu
				kinds={[
					{
						id: 'pipeline_script',
						label: 'Add downstream pipeline script',
						description: 'Triggered when this asset changes',
						icon: Code2
					}
				]}
				languages={PIPELINE_LANGUAGES as any}
				pathPrefix={data.pathPrefix ?? ''}
				defaultPathSuffix={data.defaultPathSuffix ?? ''}
				onPick={handlePick}
			>
				{#snippet trigger()}
					<!--
						Sizing notes for the round + button:
						  - w-6/h-6 (24px) chosen so that with border-2 (2px each
						    side) the inner area is exactly 20px — divisible by
						    the 16px icon to leave a 2px gap on every side. At
						    20px / 12px before, the gap was 4px-each but the
						    odd rounding interacted badly with svelte-flow's
						    fractional zoom transforms and the icon drifted
						    half a pixel off-center on certain zoom levels.
						  - `grid place-items-center` instead of flex centering:
						    flex's baseline alignment introduces a sub-pixel
						    nudge on small elements that grid avoids.
						  - `leading-none` strips the default line-height
						    contribution that the SVG inherits via the parent's
						    text rendering, otherwise the icon is shifted
						    downward by ~0.5px at fractional zooms.
					-->
					<button
						type="button"
						onclick={(e) => e.stopPropagation()}
						class="bg-surface border border-gray-400 dark:border-gray-600 text-secondary hover:bg-surface-hover rounded-full w-6 h-6 grid place-items-center shadow-sm leading-none"
						title="Add downstream pipeline script"
					>
						<Plus size={16} strokeWidth={2.5} />
					</button>
				{/snippet}
			</PipelineInsertMenu>
		</div>
	{/if}
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />

<style>
	.wm-recompute-flash {
		animation: wm-recompute-flash 1.2s ease-out 1;
	}
	@keyframes wm-recompute-flash {
		0% {
			background-color: rgba(16, 185, 129, 0);
		}
		15% {
			background-color: rgba(16, 185, 129, 0.45);
		}
		100% {
			background-color: rgba(16, 185, 129, 0);
		}
	}
</style>
