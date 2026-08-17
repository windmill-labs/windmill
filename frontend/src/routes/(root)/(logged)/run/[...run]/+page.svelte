<script lang="ts">
	import { base } from '$lib/base'
	import {
		JobService,
		type Job,
		ScriptService,
		type Script,
		type WorkflowStatus,
		type NewScript,
		ConcurrencyGroupsService,
		MetricsService,
		WorkerService,
		type ScriptArgs
	} from '$lib/gen'
	import {
		canWrite,
		computeSharableHash,
		copyToClipboard,
		encodeState,
		findMatchingCustomTag,
		getHubFlowIdFromPath,
		isHubFlowPath,
		isFlowPreview,
		isNotFlow,
		isScriptPreview
	} from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import {
		Activity,
		Calendar,
		List,
		Pen,
		RefreshCw,
		TimerOff,
		Trash,
		Code2,
		ClipboardCopy,
		GitBranch,
		EllipsisVertical,
		Share2,
		Globe,
		Users
	} from 'lucide-svelte'

	import { isJobResolvable } from '$lib/utils'
	import {
		claimRerunOrigin,
		offerToResolveOriginal,
		rememberRerunOrigin
	} from '$lib/components/runs/rerunResolution.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import DbtRunGraph from '$lib/components/dbt/DbtRunGraph.svelte'
	import DispatchEventsPanel from '$lib/components/runs/DispatchEventsPanel.svelte'
	import UpstreamSnapshotsPanel from '$lib/components/runs/UpstreamSnapshotsPanel.svelte'
	import {
		enterpriseLicense,
		initialArgsStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { ActionRow, Button, Skeleton, Tab, Alert, DrawerContent } from '$lib/components/common'
	import JobDetailHeader from '$lib/components/runs/JobDetailHeader.svelte'
	import ScriptRetryChain from '$lib/components/runs/ScriptRetryChain.svelte'
	import FlowExecutionStatus from '$lib/components/runs/FlowExecutionStatus.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import Tabs from '$lib/components/common/tabs/TabsV2.svelte'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PersistentScriptDrawer from '$lib/components/PersistentScriptDrawer.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import MemoryFootprintViewer from '$lib/components/MemoryFootprintViewer.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import Toggle from '$lib/components/Toggle.svelte'
	import WorkflowTimeline from '$lib/components/WorkflowTimeline.svelte'

	import HighlightTheme from '$lib/components/HighlightTheme.svelte'

	import ExecutionDuration from '$lib/components/ExecutionDuration.svelte'
	import { isWindmillTooBigObject } from '$lib/components/job_args'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { onDestroy, setContext, untrack } from 'svelte'
	import { getJobStatusKind, resetFavicon, setStatusFavicon } from '$lib/favicon'

	import FlowAssetsHandler, {
		initFlowGraphAssetsCtx
	} from '$lib/components/flows/FlowAssetsHandler.svelte'
	import JobAssetsViewer from '$lib/components/assets/JobAssetsViewer.svelte'
	import { page } from '$app/state'
	import { setViewToken } from '$lib/viewToken'
	import { twMerge } from 'tailwind-merge'
	import FlowRestartButton from '$lib/components/FlowRestartButton.svelte'
	import { useNestedRestartState } from '$lib/components/useNestedRestartState.svelte'
	import JobOtelTraces from '$lib/components/JobOtelTraces.svelte'
	import {
		canUserBypassRuleKind,
		isRuleActive,
		protectionRulesState
	} from '$lib/workspaceProtectionRules.svelte'
	import {
		buildForkEditUrl,
		editInForkAllowed,
		editInForkLabel,
		onEditInForkClick
	} from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	let job: (Job & { result?: any; result_stream?: string }) | undefined = $state()
	let jobUpdateLastFetch: Date | undefined = $state()

	// A re-run launched from a failed run offers to resolve that failure once it succeeds.
	// The claim happens here rather than at init because SvelteKit reuses this component
	// across /run/<id> navigations, so init only runs for the first run viewed.
	// Only ever an offer: a re-run is a fresh execution, not proof the old failure was handled.
	let offeredForJobId: string | undefined = $state(undefined)
	$effect(() => {
		if (!job || offeredForJobId === job.id) return
		if ('success' in job && job.success) {
			const origin = claimRerunOrigin(job.id)
			if (origin) {
				offeredForJobId = job.id
				offerToResolveOriginal(origin)
			}
		}
	})

	let scriptProgress: number | undefined = $state(undefined)
	let currentJobIsLongRunning: boolean = $state(false)

	let viewTab: 'logs' | 'code' | 'stats' | 'assets' | 'traces' = $state('logs')
	let selectedJobStep: string | undefined = $state(undefined)

	// Mirror of the graph's per-module display state (selected iteration of each
	// ForLoop, etc.). Drives both the iteration pre-fill and the iteration-count
	// selectors in the restart popup.
	let graphModuleStates: Record<string, import('$lib/components/graph').GraphModuleState> = $state(
		{}
	)
	let expandedSubflows: Record<
		string,
		{ modules: import('$lib/gen').FlowModule[]; groups?: any[] }
	> = $state({})
	const restart = useNestedRestartState({
		selectedJobStep: () => selectedJobStep,
		job: () => job,
		graphModuleStates: () => graphModuleStates,
		expandedSubflows: () => expandedSubflows
	})

	let testIsLoading = $state(false)
	let jobLoader: JobLoader | undefined = $state(undefined)
	let loadError: { status?: number; message?: string } | undefined = $state(undefined)

	// Flow execution status state
	let suspendStatus: import('$lib/utils').StateStore<Record<string, { job: Job; nb: number }>> =
		$state({ val: {} })
	let isOwner: boolean = $state(false)

	let persistentScriptDrawer: PersistentScriptDrawer | undefined = $state(undefined)

	let showExplicitProgressTip: boolean = $state(
		(localStorage.getItem('hideExplicitProgressTip') ?? 'false') == 'false'
	)

	let lastJobId: string | undefined = $state(undefined)
	let concurrencyKey: string | undefined = $state(undefined)

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => job?.raw_flow?.modules ?? [] })
	)

	async function getConcurrencyKey(job: Job | undefined) {
		if (!job) return
		lastJobId = job.id
		concurrencyKey = await ConcurrencyGroupsService.getConcurrencyKey({ id: job.id })
	}

	// Share read link: if the URL carries a `view_token`, install it so every job
	// read on this page (incl. flow steps, args, logs, SSE) is authorized by it.
	// Set eagerly at init (before JobLoader mounts and fires its first fetch), and
	// reactively keep it in sync across client-side navigation.
	setViewToken(page.url.searchParams.get('view_token') ?? undefined)
	$effect(() => {
		setViewToken(page.url.searchParams.get('view_token') ?? undefined)
	})
	onDestroy(() => setViewToken(undefined))

	async function shareReadLink(id: string): Promise<void> {
		try {
			const workspace = $workspaceStore!
			const token = (await JobService.getJobViewToken({ workspace, id })).trim()
			// Pin the workspace in the link: the token is signed with this workspace's
			// key, and the logged layout only switches `$workspaceStore` when the URL
			// carries `workspace=`. Without it a recipient whose active workspace
			// differs would open the run (and validate the token) against the wrong one.
			const url = `${window.location.origin}${base}/run/${id}?workspace=${encodeURIComponent(
				workspace
			)}&view_token=${encodeURIComponent(token)}`
			copyToClipboard(url)
			sendUserToast('Read-only share link copied to clipboard')
		} catch (e) {
			sendUserToast(`Failed to create share link: ${e}`, true)
		}
	}

	async function sharePublicLink(id: string): Promise<void> {
		try {
			const workspace = $workspaceStore!
			const token = (await JobService.getJobPublicViewToken({ workspace, id })).trim()
			// The workspace is a path segment here: the public run page is outside the
			// logged layout and has no workspace store to fall back on.
			const url = `${window.location.origin}${base}/public_run/${encodeURIComponent(
				workspace
			)}/${id}?view_token=${encodeURIComponent(token)}`
			copyToClipboard(url)
			sendUserToast('Public link copied to clipboard — anyone with it can view this run')
		} catch (e) {
			sendUserToast(`Failed to create public link: ${e}`, true)
		}
	}

	async function deleteCompletedJob(id: string): Promise<void> {
		await JobService.deleteCompletedJob({ workspace: $workspaceStore!, id })
		getJob()
	}

	async function cancelJob(id: string) {
		try {
			if (forceCancel) {
				await JobService.forceCancelQueuedJob({ workspace: $workspaceStore!, id, requestBody: {} })
				setTimeout(getJob, 5000)
			} else {
				await JobService.cancelQueuedJob({ workspace: $workspaceStore!, id, requestBody: {} })
			}
			sendUserToast(`job ${id} canceled`)
		} catch (err) {
			sendUserToast('could not cancel job', true)
		}
	}

	// Initialize view tab to logs since result is now outside tabs
	function initView(): void {
		// Result is now displayed outside tabs, so always default to logs
		viewTab = 'logs'
	}

	async function getJob() {
		await jobLoader?.watchJob(page.params.run ?? '', {
			change(job: Job & { result_stream?: string }) {
				// Result is now displayed outside tabs, no need to switch tabs
			},
			done(job) {
				// Result is now displayed outside tabs, no need to switch tabs
			}
		})
		initView()
	}

	let persistentScriptDefinition: Script | undefined = $state(undefined)

	async function onJobLoaded() {
		// We want to set up scriptProgress once job is loaded
		// We need this to show progress bar if job has progress and is finished
		if (
			job &&
			job.type == 'CompletedJob' &&
			(job.job_kind == 'script' || isScriptPreview(job.job_kind))
		) {
			// If error occurred and job is completed
			// than we fetch progress from server to display on what progress did it fail
			// Could be displayed after run or as a historical page
			// If opening page without running job (e.g. reloading page after run) progress will be displayed instantly
			MetricsService.getJobProgress({
				workspace: job.workspace_id ?? 'NO_WORKSPACE',
				id: job.id
			}).then((progress) => {
				// Returned progress is not always 100%, could be 65%, 33%, anything
				// Its ok if its a failure and we want to keep that value
				// But we want progress to be 100% if job has been succeeded
				scriptProgress = progress
			})
		}

		if (
			job &&
			job.job_kind === 'script' &&
			job.script_hash &&
			persistentScriptDefinition === undefined
		) {
			const script = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash: job.script_hash
			})
			if (script.restart_unless_cancelled ?? false) {
				persistentScriptDefinition = script
			}
		}
	}

	function onRunsPageChangeWithLoader() {
		forceCancel = false
		getJob()
	}

	function onRunsPageChange() {
		job = undefined
		persistentScriptDefinition = undefined
	}

	let notfound = $state(false)
	let forceCancel = $state(false)

	let debugViewer: Drawer | undefined = $state(undefined)
	let debugContent: any = $state(undefined)
	async function debugInfo() {
		if (job?.id) {
			debugContent = await JobService.getFlowDebugInfo({ workspace: $workspaceStore!, id: job?.id })
			debugViewer?.openDrawer()
		} else {
			sendUserToast('Job has no id', true)
		}
	}

	function removeSensitiveInfos(
		jobs: { [job: string]: { args: any; result: any; logs: string } },
		redactSensitive: boolean
	) {
		if (!redactSensitive) {
			return jobs
		}
		if (jobs === undefined || typeof jobs !== 'object') {
			return []
		}
		return Object.fromEntries(
			Object.entries(jobs).map(([k, job]) => {
				return [
					k,
					{
						...job,
						args: '[redacted]',
						result: '[redacted]',
						logs: '[redacted]'
					}
				]
			})
		)
	}

	let redactSensitive = $state(false)

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		if (!x || typeof x !== 'object') return {}
		const result: Record<string, WorkflowStatus> = {}
		for (const [k, v] of Object.entries(x)) {
			if (!k.startsWith('_') || k.startsWith('_step/')) result[k] = v as WorkflowStatus
		}
		return result
	}

	function getStepResults(x: any): Record<string, any> {
		return x?._checkpoint?.completed_steps ?? {}
	}

	function forkPreview() {
		if (isFlowPreview(job?.job_kind)) {
			if (isHubFlowPath(job?.script_path)) {
				const hubFlowId = getHubFlowIdFromPath(job?.script_path)
				if (hubFlowId === undefined) {
					sendUserToast('Could not determine the hub flow to fork', true)
					return
				}
				$initialArgsStore = job?.args
				window.open(`/flows/add?hub=${hubFlowId}`)
				return
			}

			const state = {
				flow: { value: job?.raw_flow },
				path: job?.script_path + '_fork',
				initialArgs: job?.args
			}
			try {
				localStorage.setItem('fork_flow', JSON.stringify(state))
			} catch {
				// Flow too large for localStorage, pass via window reference
				;(window as any).__forkPreviewData = state
			}
			window.open('/flows/add?fork=true')
		} else {
			$initialArgsStore = job?.args
			let n: NewScript = {
				path: job?.script_path + '_fork',
				summary: 'Fork of preview of ' + job?.script_path,
				language: job?.language as NewScript['language'],
				description: '',
				content: job?.raw_code ?? '',
				kind: 'script'
			}
			const encodedArgs = encodeState(job?.args)
			window.open(`/scripts/add?initial_args=${encodedArgs}#${encodeState(n)}`)
		}
	}

	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)

	// A job's stored tag is usually backend-derived (language/flow default, possibly
	// workspace-suffixed) and would be rejected by the CUSTOM_TAGS check if passed back
	// explicitly. Only carry it into a re-run when it maps back to a custom-tag entry —
	// the set the override dropdown offers — using the raw (possibly templated) entry.
	// Pass the run's args if already fetched; template matching needs the full args
	// (job.args may be truncated for large runs) and fetches them itself otherwise.
	let customTags: { workspace: string; tags: string[] } | undefined = undefined
	async function getRerunTagOverride(
		args: Record<string, any> | undefined
	): Promise<string | undefined> {
		const tag = job?.tag
		const workspace = $workspaceStore!
		if (!tag) {
			return undefined
		}
		try {
			if (customTags?.workspace !== workspace) {
				customTags = {
					workspace,
					tags: await WorkerService.getCustomTagsForWorkspace({ workspace })
				}
			}
		} catch (e) {
			console.error('Could not load custom tags, not carrying tag over for re-run', e)
			return undefined
		}
		if (customTags.tags.includes(tag)) {
			return tag
		}
		if (isWindmillTooBigObject(args)) {
			try {
				args = (await JobService.getJobArgs({ workspace, id: job?.id! })) as Record<string, any>
			} catch (e) {
				console.error('Could not load full args, not carrying tag over for re-run', e)
				return undefined
			}
		}
		return findMatchingCustomTag(tag, customTags.tags, workspace, args)
	}

	let runImmediatelyLoading = $state(false)
	// An override may be a function, because the arguments it merges into are not
	// known until they are here: a `WINDMILL_TOO_BIG` payload is a placeholder
	// until fetched, and merging over that one drops what the fetch was for.
	async function runImmediately(argsOverride?: ScriptArgs | ((args: ScriptArgs) => ScriptArgs)) {
		runImmediatelyLoading = true
		try {
			let args = job?.args as ScriptArgs
			if (isWindmillTooBigObject(args)) {
				args = (await JobService.getJobArgs({
					workspace: $workspaceStore!,
					id: job?.id!
				})) as ScriptArgs
			}
			args =
				typeof argsOverride === 'function'
					? argsOverride(args ?? {})
					: { ...args, ...(argsOverride ?? {}) }

			const commonArgs = {
				workspace: $workspaceStore!,
				requestBody: args,
				tag: await getRerunTagOverride(args)
			}
			if (job?.job_kind == 'script' || job?.job_kind == 'script_hub' || job?.job_kind == 'flow') {
				let id

				if (job?.job_kind == 'script') {
					id = await JobService.runScriptByHash({
						...commonArgs,
						hash: job.script_hash!,
						skipPreprocessor: true
					})
				} else if (job?.job_kind == 'script_hub') {
					id = await JobService.runScriptByPath({
						...commonArgs,
						path: job.script_path!,
						skipPreprocessor: true
					})
				} else {
					id = await JobService.runFlowByPath({
						...commonArgs,
						path: job.script_path!,
						skipPreprocessor: true
					})
				}

				// Offer to resolve this failure once the re-run succeeds. Captured here because the
				// new job carries no back-pointer to the run it supersedes. An already-resolved
				// failure needs no offer, and its note must not be restated as a supersession.
				if (job && isJobResolvable(job) && !job.resolved && !$userStore?.operator) {
					rememberRerunOrigin({ originalId: job.id, rerunId: id, workspace: $workspaceStore! })
				}
				await goto('/run/' + id + '?workspace=' + $workspaceStore)
			} else {
				sendUserToast('Cannot run this job immediately', true)
			}
		} catch (err) {
			// A refusal is the interesting case here: the worker rejects a `dbt
			// retry` whose run is no longer the saved one, and a caller who saw
			// nothing happen would just click again.
			sendUserToast(`Could not create job: ${err}`, true)
		} finally {
			runImmediatelyLoading = false
		}
	}

	// Whether a `dbt retry` submitted from here would resume THIS run: only the
	// latest failure of a script is kept per principal, so a later run of it — or
	// one that failed with nothing to rebuild — leaves this page's retry refused.
	// Both entry points to it, the dropdown item and the graph's banner, ask.
	let dbtResumable = $state(false)
	$effect(() => {
		const ws = $workspaceStore
		const id = job?.id
		const failed = job?.language === 'dbt' && job?.type === 'CompletedJob' && !job?.success
		dbtResumable = false
		if (!ws || !id || !failed) return
		JobService.getDbtResumable({ workspace: ws, id })
			.then((held) => {
				// The page may have moved on, or the workspace changed, in flight.
				if (job?.id === id && $workspaceStore === ws) dbtResumable = held === id
			})
			.catch(() => {})
	})

	// The retry goes through `runImmediately` so it carries the run's own arguments
	// and resolved tag: worker tags, debounce and concurrency keys interpolate from
	// the submitted arguments at enqueue, while the ones being resumed are restored
	// later, inside the worker.
	async function resumeDbtRun() {
		// Merged INTO the run's own block, not put in its place: those fields are
		// what a `$args[command.vars.tenant]` tag or concurrency key interpolates
		// from at enqueue. The worker ignores them for a retry — it rebuilds with
		// the arguments the failed run had — but the queue has already read them.
		await runImmediately((args) => ({
			...args,
			command: {
				...((args?.['command'] as Record<string, any> | undefined) ?? {}),
				label: 'retry',
				dbt_retry_job: job?.id
			}
		}))
	}

	let showEditButton = $derived(!isRuleActive('DisableDirectDeployment'))

	// Admins always pass the backend gate. Everyone else fails closed while the rulesets
	// are still loading, so the item is never briefly offered to a restricted user.
	let canSharePublicly = $derived(
		!!$userStore?.is_admin ||
			!!$userStore?.is_super_admin ||
			(protectionRulesState.rulesets !== undefined &&
				canUserBypassRuleKind('RestrictPublicRunSharing', $userStore ?? undefined))
	)

	$effect(() => {
		job?.id && lastJobId !== job.id && untrack(() => getConcurrencyKey(job))
	})
	$effect(() => {
		$workspaceStore && page.params.run && untrack(() => onRunsPageChange())
	})
	$effect(() => {
		$workspaceStore && page.params.run && jobLoader && untrack(() => onRunsPageChangeWithLoader())
	})
	$effect(() => {
		job && untrack(() => onJobLoaded())
	})
	$effect(() => {
		const status = getJobStatusKind(job)
		if (status) {
			setStatusFavicon(status)
		} else {
			resetFavicon()
		}
	})
	onDestroy(resetFavicon)
</script>

<HighlightTheme />

<ScheduleEditor bind:this={scheduleEditor} />

{#if (job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)) && job?.['running'] && job?.parent_job == undefined}
	<Drawer bind:this={debugViewer} size="800px">
		<DrawerContent title="Debug Detail" on:close={debugViewer.closeDrawer}>
			{#snippet actions()}
				<div class="flex items-center gap-1">
					<div class="w-60 pt-2">
						<Toggle bind:checked={redactSensitive} options={{ right: 'Redact args/result/logs' }} />
					</div>
					<Button
						on:click={() =>
							copyToClipboard(
								JSON.stringify(removeSensitiveInfos(debugContent, redactSensitive), null, 4)
							)}
						unifiedSize="md"
						variant="subtle"
					>
						<div class="flex gap-2 items-center">Copy <ClipboardCopy /> </div>
					</Button>
				</div>
			{/snippet}
			<pre
				><code class="text-2xs p-2">
					<Highlight
						language={json}
						code={JSON.stringify(removeSensitiveInfos(debugContent, redactSensitive), null, 4)}
					/>
			</code></pre
			>
		</DrawerContent>
	</Drawer>
{/if}
{#if !job || (job?.job_kind != 'flow' && job?.job_kind != 'flownode' && job?.job_kind != 'flowpreview')}
	<JobLoader
		bind:scriptProgress
		bind:this={jobLoader}
		bind:isLoading={testIsLoading}
		bind:job
		bind:jobUpdateLastFetch
		workspaceOverride={$workspaceStore}
		bind:notfound
		bind:loadError
	/>
{/if}

<Portal name="persistent-run">
	<PersistentScriptDrawer bind:this={persistentScriptDrawer} />
</Portal>

{#if loadError?.status === 403}
	<div class="max-w-3xl px-4 mx-auto w-full">
		<div class="mt-6">
			<Alert type="warning" title="You don't have access to this run">
				<div class="flex flex-col gap-2">
					<p>
						This run exists in <span class="font-semibold">{$workspaceStore}</span>, but you don't
						have permission to view it.
					</p>
					<p>
						Ask a colleague who can see it to open the run and use the
						<span class="font-semibold">Share</span> button to send you a read-only link. Opening that
						link will grant you access to this run (and its steps).
					</p>
				</div>
			</Alert>
			<div class="mt-4">
				<Button href="{base}/runs" unifiedSize="md" variant="accent">Go to runs page</Button>
			</div>
		</div>
	</div>
{:else if notfound || (job?.workspace_id != undefined && $workspaceStore != undefined && job?.workspace_id != $workspaceStore)}
	<div class="max-w-7xl px-4 mx-auto w-full">
		<div class="flex flex-col gap-6">
			<h1 class="text-red-400 mt-6 text-2xl font-semibold"
				>Job {page.params.run} not found in {$workspaceStore}</h1
			>
			<h2 class="text-primary text-lg font-semibold">Are you in the right workspace?</h2>
			<div class="flex flex-col gap-2">
				{#each $userWorkspaces as workspace}
					<div>
						<Button
							variant="default"
							unifiedSize="md"
							on:click={() => {
								goto(`/run/${page.params.run}?workspace=${workspace.id}`)
							}}
						>
							See in {workspace.name}
						</Button>
					</div>
				{/each}
				<div>
					<Button href="{base}/runs" unifiedSize="md" variant="accent">Go to runs page</Button>
				</div>
			</div>
		</div>
	</div>
{:else}
	<Skeleton
		class="max-w-7xl p-4 mx-auto w-full"
		loading={!job}
		layout={[
			// 1. Top Action Bar (buttons on right side)
			[
				{ h: 2.5, w: 60 },
				{ h: 2.5, w: 40 }
			],
			1,
			// 2. Job Header
			[{ h: 12, w: 100 }],
			1,
			// 3. Progress Bar
			[{ h: 2, w: 100 }],
			1.5
		]}
	/>
	<ActionRow class="max-w-7xl px-4 mx-auto w-full">
		{#snippet left()}
			<h1 class="text-sm font-semibold text-primary">run/{page.params.run}</h1>
		{/snippet}
		{#snippet right()}
			{@const isScript = job?.job_kind === 'script'}
			{@const isHubFlowPreview = isFlowPreview(job?.job_kind) && isHubFlowPath(job?.script_path)}
			{@const runsHref = `/runs/${job?.script_path}${!isScript ? '?jobKind=flow' : ''}`}
			{#if job && 'deleted' in job && !job?.deleted && ($superadmin || ($userStore?.is_admin ?? false))}
				<Dropdown
					items={[
						{
							displayName: 'Delete result, logs and args (admin only)',
							action: () => {
								job?.id && deleteCompletedJob(job.id)
							},
							type: 'delete'
						}
					]}
				>
					{#snippet buttonReplacement()}
						<Button
							nonCaptureEvent
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: Trash }}
						/>
					{/snippet}
				</Dropdown>
				{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
					<Button href={runsHref} variant="default" unifiedSize="md" startIcon={{ icon: List }}>
						View runs
					</Button>
				{/if}
			{/if}
			{#if job}
				<Dropdown
					customWidth={280}
					items={[
						{
							displayName: 'Copy link for members',
							icon: Users,
							tooltip:
								'Read-only link to this run for another member of this workspace. They must be logged in.',
							action: () => job && shareReadLink(job.id)
						},
						{
							displayName: 'Copy public link',
							icon: Globe,
							disabled: !canSharePublicly,
							tooltip: canSharePublicly
								? "Read-only link that anyone on the internet can open, without logging in. It shows a minimal version of this page: this run's inputs, result and logs, and for a flow its graph plus every step's inputs, result, logs and code. The link cannot be revoked."
								: 'Sharing a run publicly is restricted in this workspace. Ask an admin to share it, or to grant you a bypass on the ruleset.',
							action: () => job && sharePublicLink(job.id)
						}
					]}
				>
					{#snippet buttonReplacement()}
						<Button nonCaptureEvent variant="default" unifiedSize="md" startIcon={{ icon: Share2 }}>
							Share
						</Button>
					{/snippet}
				</Dropdown>
			{/if}
			{@const stem = job?.job_kind === 'script_hub' ? '/scripts' : `/${job?.job_kind}s`}
			{@const viewHref = `${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
			{#if (job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)) && job?.['running'] && job?.parent_job == undefined}
				<div class="inline">
					<Dropdown
						items={[
							{
								displayName: 'Show Flow Debug Info',
								action: () => {
									debugInfo()
								}
							}
						]}
						class="h-auto"
					>
						{#snippet buttonReplacement()}
							<Button nonCaptureEvent unifiedSize="md" variant="subtle">
								<div class="flex flex-row items-center">
									<EllipsisVertical size={14} />
								</div>
							</Button>
						{/snippet}
					</Dropdown>
				</div>
			{/if}
			{#if isFlowPreview(job?.job_kind) || isScriptPreview(job?.job_kind)}
				<Button
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: GitBranch }}
					on:click={forkPreview}
				>
					{isHubFlowPreview
						? 'Fork flow into workspace'
						: `Fork ${isFlowPreview(job?.job_kind) ? 'flow' : 'code'} preview`}
				</Button>
			{/if}
			{#if persistentScriptDefinition !== undefined}
				<Button
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: Activity }}
					on:click={() => {
						persistentScriptDrawer?.open?.(persistentScriptDefinition)
					}}
				>
					Current runs
				</Button>
			{/if}
			{#if job && job?.type != 'CompletedJob' && (!job?.schedule_path || job?.['running'] == true)}
				{#if !forceCancel}
					<Button
						unifiedSize="md"
						variant="accent"
						destructive
						startIcon={{ icon: TimerOff }}
						on:click|once={() => {
							if (job?.id) {
								cancelJob(job?.id)
								setTimeout(() => {
									forceCancel = true
								}, 3001)
							}
						}}
						title={`Cancel the ${job?.job_kind === 'script' ? 'script' : job?.job_kind === 'flow' ? 'flow' : 'job'}`}
					>
						Cancel
					</Button>
				{:else}
					<Button
						unifiedSize="md"
						variant="accent"
						destructive
						startIcon={{ icon: TimerOff }}
						on:click|once={() => {
							if (job?.id) {
								cancelJob(job?.id)
							}
						}}
					>
						Force Cancel
					</Button>
				{/if}
			{/if}
			{#if job?.schedule_path}
				<Button
					unifiedSize="md"
					variant="default"
					on:click={() => {
						if (!job || !job.schedule_path) {
							return
						}
						scheduleEditor?.openEdit(job.schedule_path, job.job_kind == 'flow')
					}}
					startIcon={{ icon: Calendar }}>Edit schedule</Button
				>
			{/if}
			{#if job?.type === 'CompletedJob' && job?.job_kind === 'flow' && selectedJobStep !== undefined && (restart.topLevelRestartable || restart.nestedRestartSupported) && job.id}
				<FlowRestartButton
					jobId={job.id}
					{selectedJobStep}
					selectedJobStepType={restart.selectedJobStepType}
					restartBranchNames={restart.restartBranchNames}
					nestedPath={restart.nestedRestartSupported ? restart.nestedRestartPath : undefined}
					nestedTopStepId={restart.nestedRestartTopStepId}
					nestedTopBranchOrIterationN={restart.nestedRestartTopBranchOrIterationN}
					presetIterationN={restart.topLevelLoopIteration}
					iterationCounts={restart.iterationCounts}
					nestedPathIterationCounts={restart.nestedPathIterationCounts}
					onRestartComplete={(newJobId) => {
						goto('/run/' + newJobId + '?workspace=' + $workspaceStore)
					}}
					flowPath={job.script_path}
					flowVersionId={job.script_hash ? parseInt(job.script_hash, 16) : undefined}
					disabled={!$enterpriseLicense}
					enterpriseOnly={!$enterpriseLicense}
				/>
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'script_hub' || job?.job_kind === 'flow'}
				<Button
					on:click|once={async () => {
						// The form this lands on rebuilds the whole project. When resuming
						// THIS run is the cheaper thing to do, name it so the form can
						// offer that in one click rather than leaving the reader to know.
						const from = dbtResumable ? `?dbt_retry_from=${job?.id}` : ''
						goto(
							viewHref +
								from +
								`#${computeSharableHash(job?.args, await getRerunTagOverride(job?.args))}`
						)
					}}
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: RefreshCw }}
					loading={runImmediatelyLoading}
					dropdownItems={[
						// A failed dbt run's cheap next step is resuming its failed and skipped
						// nodes rather than rebuilding the project, and `dbt_command` is the
						// only argument that differs. Offered first, and only while the saved
						// failure is still this run — which is what `dbtResumable` answers.
						...(dbtResumable ? [{ label: 'dbt retry with same args', onClick: resumeDbtRun }] : []),
						{
							label: 'Run immediately with same args',
							onClick: () => runImmediately()
						}
					]}
				>
					Run again
				</Button>
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
				{#if !$userStore?.operator}
					{#if canWrite(job?.script_path ?? '', {}, $userStore)}
						<Button
							href={`${stem}/edit/${job?.script_path}?workspace=${$workspaceStore}`}
							on:click={() => {
								$initialArgsStore = job?.args
							}}
							unifiedSize="md"
							variant="default"
							disabled={!showEditButton}
							size="sm"
							startIcon={{ icon: Pen }}>Edit</Button
						>
						{#if showEditButton}
							<!-- Opens the deployed runnable at this job's path, like Edit — unlike
							     "View script", which pins the hash this run executed. Same gate as
							     Edit: where direct deployment is off, the way in is "Edit in fork". -->
							<OpenInSessionButton
								source={{
									target: { kind: isScript ? 'script' : 'flow', path: job?.script_path ?? '' },
									workspaceId: $workspaceStore ?? undefined
								}}
								btnProps={{ unifiedSize: 'md' }}
							/>
						{/if}
					{/if}
					{#if !showEditButton && !isCloudHosted() && editInForkAllowed($workspaceStore, $userWorkspaces)}
						<Button
							href={buildForkEditUrl(isScript ? 'script' : 'flow', job?.script_path ?? '')}
							onClick={(e) =>
								onEditInForkClick(e, isScript ? 'script' : 'flow', job?.script_path ?? '', {
									hasHref: true
								})}
							unifiedSize="md"
							variant="default"
							size="sm"
							startIcon={{ icon: Pen }}>{editInForkLabel($workspaceStore, $userWorkspaces)}</Button
						>
					{/if}
				{/if}
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'script_hub' || job?.job_kind === 'flow'}
				<Button
					href={viewHref}
					unifiedSize="md"
					variant="accent"
					startIcon={{
						icon:
							job?.job_kind === 'script' || job?.job_kind === 'script_hub'
								? Code2
								: job?.job_kind === 'flow'
									? BarsStaggered
									: Code2
					}}
				>
					View {job?.job_kind === 'script_hub' ? 'script' : job?.job_kind}
				</Button>
			{/if}
		{/snippet}
	</ActionRow>
	<div class={twMerge('w-full', isNotFlow(job?.job_kind) && 'pb-8')}>
		<!-- Flow Detail Header Card -->
		<div class="max-w-7xl mx-auto px-4 py-0">
			<Skeleton loading={!job} layout={[[24]]} />
			{#if job}
				<JobDetailHeader
					{job}
					{scheduleEditor}
					displayPersistentScriptDefinition={!!persistentScriptDefinition}
					openPersistentScriptDrawer={() => {
						persistentScriptDrawer?.open?.(persistentScriptDefinition)
					}}
					{concurrencyKey}
				/>
			{/if}
		</div>
		{#if job?.['deleted']}
			<div class="max-w-7xl mx-auto w-full px-4 mt-6">
				<Alert type="error" title="Deleted">
					The content of this run was deleted (by an admin, no less)
				</Alert>
			</div>
			<div class="my-4"></div>
		{/if}

		<!-- Flow Progress Bar (for flows only) -->
		{#if job?.job_kind === 'flow' || job?.job_kind === 'flowpreview'}
			<div class="max-w-7xl mx-auto w-full px-4 flex flex-col gap-4 mt-2">
				<FlowProgressBar
					{job}
					bind:currentSubJobProgress={scriptProgress}
					class="w-full"
					textPosition="bottom"
					slim
					showStepId
				/>
				{#if suspendStatus}
					<FlowExecutionStatus
						{job}
						{isOwner}
						{suspendStatus}
						innerModules={job?.flow_status?.modules}
					/>
				{/if}
			</div>
		{/if}

		<!-- Arguments and actions -->
		<div class="max-w-7xl mx-auto w-full px-4 mt-12">
			<div class="text-xs text-emphasis font-semibold mb-1">Inputs</div>
			<div class="flex flex-col gap-y-6">
				<JobArgs
					workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
					id={job?.id}
					args={job?.args}
				/>
				{#if job && currentJobIsLongRunning && showExplicitProgressTip && !scriptProgress && 'running' in job}
					<Alert
						class="p-1 flex flex-row relative text-center"
						size="xs"
						type="info"
						title="tip: Track progress of longer jobs"
						tooltip="For better transparency and verbosity, you can try setting progress from within the script."
						documentationLink="https://www.windmill.dev/docs/advanced/explicit_progress"
					>
						<button
							type="button"
							onclick={() => {
								localStorage.setItem('hideExplicitProgressTip', 'true')
								showExplicitProgressTip = false
							}}
							class="absolute m-2 top-0 right-0 inline-flex rounded-md bg-surface-secondary text-primary hover:text-primary focus:outline-none"
						>
							<span class="sr-only">Close</span>
							<svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
								<path
									d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z"
								/>
							</svg>
						</button>
					</Alert>
				{/if}
			</div>
		</div>

		{#if job}
			<ScriptRetryChain {job} />
		{/if}

		{#if isNotFlow(job?.job_kind)}
			{#if ['python3', 'bun', 'deno'].includes(job?.language ?? '') && (job?.job_kind == 'script' || isScriptPreview(job?.job_kind))}
				<ExecutionDuration {job} bind:longRunning={currentJobIsLongRunning} />
			{/if}
			<div class="max-w-7xl mx-auto w-full px-4 mb-10">
				{#if job?.workflow_as_code_status && job.job_kind !== 'aiagent'}
					<div class="mr-2 sm:mr-0 mt-12 mb-6">
						<h3 class="text-xs font-semibold text-emphasis mb-1">Workflow Timeline</h3>
						<div class="border rounded-md overflow-hidden">
							<WorkflowTimeline
								flow_status={asWorkflowStatus(job.workflow_as_code_status)}
								flowDone={job.type == 'CompletedJob'}
								stepResults={getStepResults(job.workflow_as_code_status)}
								result={job.result}
								success={(job as any).success !== false}
								jobId={job.id}
							/>
						</div>
					</div>
				{/if}
				{#if scriptProgress}
					<JobProgressBar {job} {scriptProgress} class="py-4" hideStepTitle={true} />
				{/if}

				<!-- The models a dbt run touches, above its result: the run page is
				     where you land on a running job, and the per-node table below
				     only exists once the job has produced one. -->
				{#if job?.language === 'dbt' && job?.script_path}
					<div class="mr-2 sm:mr-0 mt-12">
						<h3 class="text-xs font-semibold text-emphasis mb-1">Models</h3>
						<DbtRunGraph
							scriptPath={job.script_path}
							jobId={job.id}
							running={job.type !== 'CompletedJob'}
							result={job.type === 'CompletedJob' ? job.result : undefined}
							scriptHash={job.script_hash}
							runArgs={job.args}
							canResume={dbtResumable}
							onResume={resumeDbtRun}
						/>
					</div>
				{/if}

				<!-- Result Section (moved outside tabs) -->
				{#if job}
					<div class="mr-2 sm:mr-0 mt-12 mb-6">
						<h3 class="text-xs font-semibold text-emphasis mb-1">Result</h3>
						<div class="border rounded-md bg-surface-tertiary p-4 overflow-auto max-h-screen">
							{#if job.result_stream || (job.type == 'CompletedJob' && job.result !== undefined)}
								<DisplayResult
									workspaceId={job?.workspace_id}
									result_stream={job.result_stream}
									jobId={job?.id}
									result={job.result}
									language={job?.language}
									isTest={false}
								/>
							{:else}
								<div class="text-secondary text-sm">No output is available yet</div>
							{/if}
						</div>
					</div>
					{#if job.id && job.workspace_id}
						<UpstreamSnapshotsPanel args={job.args} />
						<DispatchEventsPanel workspace={job.workspace_id} jobId={job.id} />
					{/if}
				{/if}

				<!-- Logs and outputs-->
				<div class="mr-2 sm:mr-0 mt-6">
					<Tabs bind:selected={viewTab}>
						<Tab value="logs" label="Logs" />
						<Tab value="stats" label="Metrics" />
						<Tab value="traces" label="Traces" />
						<Tab value="assets" label="Assets" />
						{#if isScriptPreview(job?.job_kind)}
							<Tab value="code" label="Code" />
						{/if}
					</Tabs>

					<Skeleton loading={!job} layout={[[5]]} />
					{#if job}
						<div
							class={twMerge(
								'flex flex-row border rounded-md p-2 mt-2 overflow-auto min-h-[600px]',
								viewTab == 'logs' ? 'bg-surface-secondary' : 'bg-surface-tertiary'
							)}
						>
							{#if viewTab == 'logs'}
								<div class="w-full">
									<LogViewer
										jobId={job.id}
										duration={job?.['duration_ms']}
										mem={job?.['mem_peak']}
										isLoading={job?.['running'] == false}
										content={job?.logs}
										tag={job?.tag}
									/>
								</div>
							{:else if viewTab == 'assets'}
								<div class="w-full">
									<JobAssetsViewer {job} />
								</div>
							{:else if viewTab == 'traces'}
								<div class="w-full">
									<JobOtelTraces jobId={job.id} />
								</div>
							{:else if viewTab == 'code'}
								{#if job && 'raw_code' in job && job.raw_code}
									<div class="text-xs">
										<HighlightCode lines language={job.language} code={job.raw_code} />
									</div>
								{:else if job}
									<span class="text-sm">No code available</span>
								{:else}
									<Skeleton layout={[[5]]} />
								{/if}
							{:else if viewTab == 'stats'}
								<div class="w-full">
									<MemoryFootprintViewer jobId={job.id} {jobUpdateLastFetch} />
								</div>
							{:else}
								<div class="w-full p-4 text-secondary">Select a tab to view content</div>
							{/if}
						</div>
					{/if}
				</div>
			</div>
		{:else if !job?.['deleted']}
			<div class="mt-10"></div>

			<div class="w-full mt-10">
				{#if job?.id}
					<FlowStatusViewer
						jobId={job?.id ?? ''}
						onJobsLoaded={({ job: newJob }) => {
							job = newJob
						}}
						onDone={({ job: newJob }) => {
							job = newJob
						}}
						initialJob={job}
						workspaceId={$workspaceStore}
						bind:selectedJobStep
						bind:suspendStatus
						bind:isOwner
						bind:localModuleStates={graphModuleStates}
						bind:expandedSubflows
					/>
				{:else}
					<Skeleton layout={[[5]]} />
				{/if}
			</div>
		{/if}
	</div>
{/if}

<FlowAssetsHandler
	modules={job?.raw_flow?.modules ?? []}
	enableDbExplore
	enablePathScriptAndFlowAssets
/>
