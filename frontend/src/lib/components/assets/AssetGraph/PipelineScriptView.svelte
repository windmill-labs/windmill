<script lang="ts">
	import type { Script } from '$lib/gen'
	import { Loader2, Play, Upload, Zap } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import AssetRunsPanel from './AssetRunsPanel.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { emptySchema } from '$lib/utils'

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
		onRunCompleted
	}: Props = $props()

	let args = $state<Record<string, any>>({})
	let isValid = $state(true)
	let running = $state(false)
	// Offer "Run + downstream" only when a cascade dispatch is wired (dev preview)
	// and the script actually has subscribers to fan out to.
	let hasCascade = $derived(!!onRunCascade && downstreamCount > 0)

	// Local clone for SchemaForm's bindable schema prop — the incoming
	// script is owned by the parent and must not be mutated from here.
	// Seeded once: the details pane keys this component on script.path, so
	// switching scripts remounts with a fresh clone.
	// svelte-ignore state_referenced_locally
	let formSchema = $state<any>(structuredClone($state.snapshot(script.schema) ?? emptySchema()))

	async function run(cascade = false) {
		const dispatch = cascade ? onRunCascade : onRun
		if (!dispatch || running) return
		running = true
		try {
			await dispatch(script.path, $state.snapshot(args))
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
				{#if canRun}
					<div class="shrink-0 flex flex-col gap-2 p-3 border-b" data-run-form>
						<div class="flex items-center justify-between gap-2">
							<span class="text-xs font-semibold text-emphasis inline-flex items-center gap-1.5">
								<Upload size={13} class="text-fuchsia-600 dark:text-fuchsia-400" />
								Run pipeline
							</span>
							<div class="flex items-center gap-1.5">
								{#if hasCascade}
									<Button
										variant="accent-secondary"
										unifiedSize="sm"
										startIcon={{ icon: running ? Loader2 : Zap }}
										onclick={() => run(true)}
										disabled={isDraft || running || !isValid}
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
									disabled={isDraft || running || !isValid || !onRun}
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
						<SchemaForm bind:schema={formSchema} bind:args bind:isValid compact />
					</div>
				{/if}
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
