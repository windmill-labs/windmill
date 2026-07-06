<script lang="ts">
	import type { AssetKind, Script } from '$lib/gen'
	import { CalendarClock, Loader2, Play, Upload, Zap } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import PipelineRunForm from './PipelineRunForm.svelte'
	import AssetRunsPanel from './AssetRunsPanel.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { workspaceStore } from '$lib/stores'
	import { parsePipelineAnnotations } from './parsePipelineAnnotations'

	interface Props {
		script: Script
		// Drafts-overlay draft: rendered read-only with a "not deployed"
		// banner; the run affordance is disabled because legitimate runs
		// only execute deployed code.
		isDraft?: boolean
		// The script is a data-upload pipeline entry point — show the run
		// form (S3 picker comes from the script's declared S3Object input).
		canRun?: boolean
		// Legitimate dispatch (runScriptByPath, real cascade) owned by the
		// page. Returns the launched job id.
		onRun?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		// Run this script AND its downstream closure with the same form args (dev
		// preview: the client orchestrates the chain). When unset — the deployed
		// pane — the single `onRun` already cascades via the backend dispatcher, so
		// no separate "Run + downstream" affordance is shown.
		onRunCascade?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		// Downstream subscriber count; the cascade button is offered only when > 0.
		downstreamCount?: number
		// Threaded through to AssetRunsPanel — same semantics as the asset
		// selection branch of the details pane.
		runsRefreshKey?: any
		runsPendingJobId?: string | undefined
		onRunCompleted?: () => void
		// Seed the run form with previously-staged args (e.g. a data-upload entry
		// whose S3Object was picked earlier and persisted at the page level). Lets
		// a re-opened node show the file it already has instead of a blank form.
		initialArgs?: Record<string, any>
		// Emitted whenever the run form's args change, so the page can persist a
		// data-upload entry's staged input — that drives the node's "green / ready"
		// state and seeds the whole-pipeline run.
		onArgsChange?: (path: string, args: Record<string, any>) => void
	}

	let {
		script,
		isDraft = false,
		canRun = false,
		onRun,
		onRunCascade,
		downstreamCount = 0,
		runsRefreshKey,
		runsPendingJobId,
		onRunCompleted,
		initialArgs,
		onArgsChange
	}: Props = $props()

	// Seed once from the persisted args (see initialArgs). Cloned so the run
	// form mutates its own copy, not the page's stored snapshot.
	// svelte-ignore state_referenced_locally
	let args = $state<Record<string, any>>(
		initialArgs ? structuredClone($state.snapshot(initialArgs)) : {}
	)
	// Push args back up so the page can persist a staged data-upload entry.
	// Reading the deep snapshot tracks nested changes (e.g. an S3Object's `s3`
	// field, or items added to a required array).
	$effect(() => {
		onArgsChange?.(script.path, $state.snapshot(args))
	})
	let isValid = $state(true)
	let running = $state(false)
	// Offer "Run + downstream" only when a cascade dispatch is wired (dev preview)
	// and the script actually has subscribers to fan out to.
	let hasCascade = $derived(!!onRunCascade && downstreamCount > 0)

	// Parse the pipeline header for the partition cadence + materialize target +
	// ducklake inputs, so a partitioned producer's run form swaps the bare
	// `partition` string for a cadence-aware date picker with missing/upstream
	// hints (see PartitionArgControl).
	let annotations = $derived(parsePipelineAnnotations(script.content ?? ''))
	let partitionSpec = $derived(annotations.partition)
	let materializeTarget = $derived<{ kind: AssetKind; path: string } | undefined>(
		annotations.materialize
			? { kind: annotations.materialize.targetKind, path: annotations.materialize.targetPath }
			: undefined
	)
	let upstreamAssets = $derived(
		annotations.triggerAssets.map((a) => ({ kind: a.kind, path: a.path }))
	)

	// Whether the script declares any inputs — a run then needs the SchemaForm
	// to collect them, so the form must render even for a plain (non-upload,
	// non-partitioned) script rather than firing with empty args.
	let hasInputs = $derived(Object.keys(script.schema?.properties ?? {}).length > 0)
	// The run form appears for data-upload entry points (S3 picker), for any
	// partitioned producer (materialize a single partition — available in OSS,
	// and the picker is where the user chooses which one), and whenever the
	// script takes inputs the run needs.
	let showRunForm = $derived(canRun || !!partitionSpec || hasInputs)
	// A partitioned producer that isn't a data-upload entry is materializing a
	// partition rather than uploading data — reflect that in the header.
	let partitionOnly = $derived(!canRun && !!partitionSpec)
	// `isValid` is only meaningful while a form is mounted to vouch for it; with
	// no form the run fires with `{}`, which is always valid. Deriving this rather
	// than reading `isValid` directly avoids a stuck-disabled Run button when a
	// required-input form leaves `isValid=false` behind as it unmounts on a
	// same-path re-resolve to an input-less script (we're keyed on script.path).
	let runValid = $derived(showRunForm ? isValid : true)

	// The details pane keys this component on script.path only, so a same-path
	// re-resolve (in /pipeline_dev the selected node re-resolves on every WS
	// bundle) does NOT remount us. `PipelineRunForm` owns the SchemaForm clone and
	// is keyed on the serialized schema + partition spec below: a local edit that
	// adds/removes args OR changes the `// partitioned` header (which the schema
	// alone wouldn't capture) reseeds the run form and re-strips the `partition`
	// field, while an unchanged re-resolve keeps in-progress input (else the form
	// could run against a stale schema/spec — e.g. keep a pre-`start=` bucket).
	let schemaKey = $derived(
		JSON.stringify(script.schema ?? null) + '|' + JSON.stringify(partitionSpec ?? null)
	)

	async function run(cascade = false) {
		const dispatch = cascade ? onRunCascade : onRun
		if (!dispatch || running) return
		running = true
		try {
			// No form rendered ⇒ no inputs to collect: run with {} rather than args a
			// prior (input-carrying) resolve of this same path left behind unmounted.
			await dispatch(script.path, showRunForm ? $state.snapshot(args) : {})
		} finally {
			running = false
		}
	}
</script>

<div class="flex flex-col h-full">
	{#if isDraft}
		<div
			class="shrink-0 flex items-center gap-2 px-3 py-1.5 text-2xs bg-amber-50 dark:bg-amber-900/30 border-b border-amber-200 dark:border-amber-900/60 text-amber-700 dark:text-amber-400"
		>
			Draft — not deployed. This is what the script will look like once deployed.
		</div>
	{/if}
	<Splitpanes horizontal class="!h-full">
		<Pane size={55} minSize={20}>
			<div class="flex flex-col h-full overflow-auto">
				<!-- Run affordance is always the first thing in the node panel so
				     it's discoverable the moment a script is selected — never
				     buried below the source. The parameter form renders under it
				     only when the run actually needs inputs (upload/partition/args). -->
				<div class="shrink-0 flex flex-col gap-2 p-3 border-b" data-run-form>
					<div class="flex items-center justify-between gap-2">
						<span class="text-xs font-semibold text-emphasis inline-flex items-center gap-1.5">
							{#if partitionOnly}
								<CalendarClock size={13} class="text-fuchsia-600 dark:text-fuchsia-400" />
								Materialize a partition
							{:else if canRun}
								<Upload size={13} class="text-fuchsia-600 dark:text-fuchsia-400" />
								Run pipeline
							{:else}
								<Play size={13} class="text-fuchsia-600 dark:text-fuchsia-400" />
								Run this step
							{/if}
						</span>
						<div class="flex items-center gap-1.5">
							{#if hasCascade}
								<Button
									variant="accent-secondary"
									unifiedSize="sm"
									startIcon={{ icon: running ? Loader2 : Zap }}
									onclick={() => run(true)}
									disabled={isDraft || running || !runValid}
									title={`Run this script with these inputs, then run its ${downstreamCount} downstream pipeline script${downstreamCount === 1 ? '' : 's'} in order`}
								>
									Run + downstream
								</Button>
							{/if}
							<Button
								variant="accent"
								unifiedSize="sm"
								startIcon={{ icon: running ? Loader2 : Play }}
								onclick={() => run(false)}
								disabled={isDraft || running || !runValid || !onRun}
								title={isDraft
									? 'Deploy this draft to run it for real'
									: hasCascade
										? 'Run just this step — downstream scripts are not triggered'
										: 'Run the deployed script — downstream pipeline scripts cascade for real'}
							>
								{running ? 'Starting…' : 'Run'}
							</Button>
						</div>
					</div>
					{#if showRunForm}
						{#key schemaKey}
							<PipelineRunForm
								schema={script.schema}
								bind:args
								bind:isValid
								{partitionSpec}
								workspace={$workspaceStore ?? ''}
								{materializeTarget}
								{upstreamAssets}
							/>
						{/key}
					{/if}
				</div>
				<div class="flex-1 min-h-0 overflow-auto text-xs p-3">
					<HighlightCode code={script.content ?? ''} language={script.language} />
				</div>
			</div>
		</Pane>
		<Pane size={45} minSize={20}>
			<AssetRunsPanel
				producers={[{ kind: 'script', path: script.path }]}
				refreshKey={runsRefreshKey}
				pendingJobId={runsPendingJobId}
				{onRunCompleted}
			/>
		</Pane>
	</Splitpanes>
</div>
