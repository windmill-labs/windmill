<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatShortAssetPath, type AssetKind } from '$lib/components/assets/lib'
	import { NODE } from '$lib/components/graph/util'
	import PipelineInsertMenu, { type PipelineInsertPick } from './PipelineInsertMenu.svelte'
	import { ArrowUpRight, Code2, GitFork, Play, Loader2, Plus } from 'lucide-svelte'
	import type { ScriptLang } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
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
		<!-- Fork data-environment marker: amber ↗ = deferred (reads the parent
		     workspace's current table via a view), emerald fork glyph = the fork
		     materialized its own copy. Icon-only — the pill truncates its path
		     already, a labeled chip wouldn't fit; the title carries the meaning. -->
		{#if data.fork_materialization === 'deferred'}
			<span
				class="shrink-0 mr-1.5 text-amber-600 dark:text-amber-400"
				title="Deferred to parent workspace: reads the parent's current data. Materialize it in this fork to iterate on it."
			>
				<ArrowUpRight size={12} />
			</span>
		{:else if data.fork_materialization === 'fork'}
			<span
				class="shrink-0 mr-1.5 text-emerald-600 dark:text-emerald-500"
				title="Materialized in this fork: reads and writes use the fork's isolated copy."
			>
				<GitFork size={12} />
			</span>
		{/if}
	</div>
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
