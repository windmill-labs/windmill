<script lang="ts">
	import { wsBase } from '$lib/workspaceUrl'
	import { displayDate, msToSec, emptyString } from '$lib/utils'
	import { onDestroy } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'
	import { ChevronDown, ChevronRight, Loader2, Moon, ShieldCheck } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'
	import LogViewer from './LogViewer.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { CheckCircle2, XCircle } from 'lucide-svelte'
	import { JobService, type Job, type WorkflowStatus } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { Button } from '$lib/components/common'
	import { Alert } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		flow_status: Record<string, WorkflowStatus>
		flowDone?: boolean
		stepResults?: Record<string, any>
		result?: any
		success?: boolean
		autoExpandResult?: boolean
		jobId?: string
	}

	let {
		flow_status,
		flowDone = false,
		stepResults = {},
		result = undefined,
		success = true,
		autoExpandResult = false,
		jobId = undefined
	}: Props = $props()

	let resultExpanded = $state(false)

	// Auto-expand result row when job completes (only if requested)
	$effect(() => {
		if (autoExpandResult && flowDone && result !== undefined) {
			resultExpanded = true
		}
	})

	let now = $state(getDbClockNow().getTime())

	let interval = setInterval(() => {
		if (!max) {
			now = getDbClockNow().getTime()
		}
	}, 30)

	onDestroy(() => {
		interval && clearInterval(interval)
		pollInterval && clearInterval(pollInterval)
	})

	let min = $derived(
		Object.values(flow_status).reduce(
			(a, b) => Math.min(a, b.scheduled_for ? new Date(b.scheduled_for).getTime() : Infinity),
			Infinity
		)
	)
	let max = $derived(
		flowDone
			? Object.values(flow_status).reduce((a, b) => {
					if (!b.started_at) return a
					const startedAt = new Date(b.started_at).getTime()
					// For cancelled steps without duration_ms, use `now` as end time
					const endAt = b.duration_ms != undefined ? startedAt + b.duration_ms : now
					return Math.max(a, endAt)
				}, 0)
			: undefined
	)
	let total = $derived(flowDone && max ? max - min : Math.max(now - min, 2000))

	// Collapsible state
	let expandedRows: Record<string, boolean> = $state({})
	let childJobs: Record<string, Job & { result?: any }> = $state({})
	let loadingJobs: Record<string, boolean> = $state({})

	function isStep(key: string): boolean {
		return key.startsWith('_step/')
	}

	function stepKey(key: string): string {
		return key.slice('_step/'.length)
	}

	function toggleRow(id: string) {
		expandedRows[id] = !expandedRows[id]
		if (expandedRows[id] && !isStep(id) && !childJobs[id]) {
			fetchChildJob(id)
		}
	}

	async function fetchChildJob(id: string) {
		const ws = $workspaceStore
		if (!ws) return
		loadingJobs[id] = true
		try {
			const job = await JobService.getJob({ workspace: ws, id })
			childJobs[id] = job as Job & { result?: any }
		} catch (e) {
			console.error(`Failed to fetch job ${id}:`, e)
		} finally {
			loadingJobs[id] = false
		}
	}

	// Poll for updates on expanded in-progress jobs
	let pollInterval = setInterval(() => {
		for (const [id, v] of Object.entries(flow_status)) {
			if (isStep(id)) continue
			const isRunning = !flowDone && v.duration_ms == undefined && v.started_at != undefined
			if (expandedRows[id] && isRunning) {
				fetchChildJob(id)
			}
		}
	}, 2000)

	// Approval action state
	let approvalLoading: Record<string, boolean> = $state({})
	let approvalFormArgs: Record<string, Record<string, any>> = $state({})

	async function handleApprove(key: string, formSchema: any) {
		const ws = $workspaceStore
		if (!ws || !jobId) return
		approvalLoading[key] = true
		try {
			const payload =
				formSchema && Object.keys(formSchema).length > 0 ? (approvalFormArgs[key] ?? {}) : undefined
			await JobService.resumeSuspended({
				workspace: ws,
				jobId: jobId,
				requestBody: { payload, approved: true }
			})
			sendUserToast('Approval submitted')
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to approve', true)
		} finally {
			approvalLoading[key] = false
		}
	}

	let cancelLoading = $state(false)
	async function handleCancel() {
		const ws = $workspaceStore
		if (!ws || !jobId) return
		cancelLoading = true
		try {
			await JobService.resumeSuspended({
				workspace: ws,
				jobId: jobId,
				requestBody: { approved: false }
			})
			sendUserToast('Job cancelled')
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to cancel', true)
		} finally {
			cancelLoading = false
		}
	}
</script>

{#if flow_status}
	<div class="divide-y border-b">
		<div class="px-2 py-2 grid grid-cols-12 w-full">
			<div></div>
			<div class="col-span-11 pt-1 px-2 flex text-2xs text-secondary justify-between">
				<div>{min ? displayDate(new Date(min), true) : ''}</div>
				{#if max && min}
					<div class="hidden lg:block">{msToSec(max - min)}s</div>
				{/if}
				<div class="flex gap-1 items-center font-mono">
					{max ? displayDate(new Date(max), true) : ''}
					{#if !max && min}
						{#if now}
							{msToSec(now - min, 3)}s
						{/if}
						<Loader2 size={14} class="animate-spin" />
					{/if}
				</div>
			</div>
		</div>
		<div class="flex flex-row-reverse items-center text-sm text-secondary p-2">
			<div class="flex gap-4 items-center text-2xs">
				<div class="flex gap-2 items-center">
					<div>Waiting for executor</div>
					<div class="h-4 w-4 bg-gray-500"></div>
				</div>
				<div class="flex gap-2 items-center">
					<div>Execution</div>
					<div class="h-4 w-4 bg-blue-500/90"></div>
				</div>
			</div>
		</div>
		{#each Object.entries(flow_status).sort(([, a], [, b]) => {
			const ta = new Date(a.started_at ?? a.scheduled_for ?? 0).getTime()
			const tb = new Date(b.started_at ?? b.scheduled_for ?? 0).getTime()
			return ta - tb
		}) as [k, v] (k)}
			{@const isInlineStep = isStep(k)}
			{@const isSleep = (v as any).sleep_duration_s != undefined}
			{@const isApproval = (v as any).approval === true}
			{@const isRunning = !flowDone && v.duration_ms == undefined && v.started_at != undefined}
			{@const isDone = v.duration_ms != undefined || flowDone}
			{@const isExpanded = expandedRows[k] ?? false}
			<div class="border-b last:border-b-0">
				{#if isSleep}
					<div class="w-full px-2 py-1.5 text-xs flex items-center gap-2 text-tertiary">
						<div class="w-3 flex-shrink-0"></div>
						<Moon size={12} class="flex-shrink-0" />
						<span class="italic">sleep ({(v as any).sleep_duration_s}s)</span>
					</div>
				{:else if isApproval}
					{@const selfApprovalDisabled = (v as any).self_approval_disabled === true}
					{@const formSchema = (v as any).form?.schema ?? (v as any).form}
					{@const hasForm =
						formSchema && typeof formSchema === 'object' && Object.keys(formSchema).length > 0}
					{@const canApprove = !isDone && jobId}
					<div class="w-full px-2 py-1.5 text-xs">
						<div class="flex items-center gap-2">
							<div class="w-3 flex-shrink-0"></div>
							<ShieldCheck
								size={12}
								class="flex-shrink-0 {isDone ? 'text-green-600' : 'text-yellow-500'}"
							/>
							<span class="italic text-secondary">
								{v.name ?? stepKey(k)}
							</span>
							{#if !isDone}
								<span class="text-tertiary flex items-center gap-1">
									<Loader2 size={12} class="animate-spin" />
									waiting
								</span>
								{#if canApprove}
									<div class="ml-auto flex gap-1">
										<Button
											variant="default"
											unifiedSize="sm"
											disabled={approvalLoading[k]}
											onclick={() => handleApprove(k, formSchema)}
										>
											Approve
										</Button>
										<Button
											variant="default"
											unifiedSize="sm"
											disabled={approvalLoading[k] || cancelLoading}
											onclick={() => handleCancel()}
										>
											Reject
										</Button>
									</div>
								{/if}
							{:else}
								<span class="text-tertiary">{msToSec(v.duration_ms ?? 0)}s</span>
							{/if}
						</div>
						{#if canApprove && selfApprovalDisabled && $userStore?.is_admin}
							<div class="mt-1 ml-5 text-yellow-600 text-2xs">
								Self-approval is disabled but allowed because you are an admin/owner
							</div>
						{/if}
						{#if canApprove && hasForm}
							<div class="mt-2 ml-5 max-w-md">
								{#if emptyString($enterpriseLicense)}
									<Alert
										type="warning"
										title="Adding a form to the approval page is an EE feature"
									/>
								{:else}
									<SchemaForm
										onlyMaskPassword
										noVariablePicker
										schema={{
											properties: formSchema,
											order: Object.keys(formSchema)
										}}
										bind:args={approvalFormArgs[k]}
									/>
								{/if}
							</div>
						{/if}
					</div>
				{:else}
					<button
						class="w-full px-2 py-2 text-xs hover:bg-surface-hover cursor-pointer flex items-center gap-2"
						onclick={() => toggleRow(k)}
					>
						<div class="inline-flex gap-1 items-center flex-shrink-0">
							<div class="w-3 flex-shrink-0">
								{#if isExpanded}
									<ChevronDown size={12} />
								{:else}
									<ChevronRight size={12} />
								{/if}
							</div>
							{#if isInlineStep}
								<span class="whitespace-nowrap text-secondary italic">
									{v.name ?? stepKey(k)}
								</span>
							{:else}
								<a
									target="_blank"
									class="inline-flex gap-2 items-baseline hover:underline whitespace-nowrap"
									href="{$wsBase}/run/{k}"
									onclick={(e) => e.stopPropagation()}
								>
									{v.name ?? k}
								</a>
							{/if}
						</div>
						<div class="flex items-center pt-1 min-h-6 w-full min-w-0">
							{#if min && total}
								{@const scheduledFor = v?.scheduled_for
									? new Date(v?.scheduled_for).getTime()
									: undefined}
								{@const startedAt = v?.started_at ? new Date(v?.started_at).getTime() : undefined}
								{@const waitingLen = scheduledFor
									? startedAt
										? startedAt - scheduledFor
										: now - scheduledFor
									: 0}

								<div class="flex w-full">
									{#if isInlineStep}
										<!-- Inline steps have no waiting phase -->
										{#if startedAt}
											<TimelineBar
												position="center"
												id={k}
												{total}
												{min}
												started_at={startedAt}
												len={v.duration_ms ?? now - startedAt}
												running={!flowDone && v.duration_ms == undefined}
											/>
										{/if}
									{:else}
										<TimelineBar
											position="left"
											id={k}
											{total}
											{min}
											gray
											started_at={scheduledFor}
											len={waitingLen < 100 ? 0 : waitingLen - 100}
											running={!flowDone && startedAt == undefined}
										/>
										{#if startedAt}
											<TimelineBar
												position={waitingLen < 100 ? 'center' : 'right'}
												id={k}
												{total}
												{min}
												concat
												started_at={startedAt}
												len={v.duration_ms ?? now - startedAt}
												running={!flowDone && v.duration_ms == undefined}
											/>
										{/if}
									{/if}
								</div>
							{/if}
						</div>
					</button>

					{#if isExpanded}
						<div class="border-t bg-surface-secondary px-4 py-2">
							{#if isInlineStep}
								<!-- Inline step: show result from checkpoint -->
								{@const result = stepResults[stepKey(k)]}
								{#if isDone && result !== undefined}
									<div>
										<div class="text-2xs text-secondary font-semibold mb-1"> Result </div>
										<div class="max-h-40 overflow-auto">
											<ObjectViewer json={result} pureViewer />
										</div>
									</div>
								{:else}
									<div class="text-xs text-secondary py-1"> Step completed (no result) </div>
								{/if}
							{:else if loadingJobs[k] && !childJobs[k]}
								<div class="flex items-center gap-2 text-xs text-secondary py-1">
									<Loader2 size={12} class="animate-spin" />
									Loading...
								</div>
							{:else if childJobs[k]}
								{@const job = childJobs[k]}
								<!-- Logs -->
								{#if job.logs || isRunning}
									<div class="mb-2">
										<div class="text-2xs text-secondary font-semibold mb-1"> Logs </div>
										<LogViewer
											content={job.logs ?? ''}
											jobId={k}
											isLoading={isRunning}
											small
											tag={job.tag}
											download={false}
											noMaxH={false}
										/>
									</div>
								{/if}

								<!-- Result -->
								{#if isDone && job.result !== undefined}
									<div>
										<div class="text-2xs text-secondary font-semibold mb-1"> Result </div>
										<div class="max-h-40 overflow-auto">
											<ObjectViewer json={job.result} pureViewer />
										</div>
									</div>
								{/if}
							{:else}
								<div class="text-xs text-secondary py-1">No data available</div>
							{/if}
						</div>
					{/if}
				{/if}
			</div>
		{/each}
		{#if flowDone && result !== undefined}
			<div class="border-b last:border-b-0">
				<button
					class="w-full px-2 py-2 text-xs hover:bg-surface-hover cursor-pointer flex items-center gap-2"
					onclick={() => (resultExpanded = !resultExpanded)}
				>
					<div class="inline-flex gap-1 items-center flex-shrink-0">
						<div class="w-3 flex-shrink-0">
							{#if resultExpanded}
								<ChevronDown size={12} />
							{:else}
								<ChevronRight size={12} />
							{/if}
						</div>
						<span class="whitespace-nowrap font-semibold flex items-center gap-1">
							{#if success}
								<CheckCircle2 size={12} class="text-green-600" />
							{:else}
								<XCircle size={12} class="text-red-600" />
							{/if}
							Result
						</span>
					</div>
				</button>
				{#if resultExpanded}
					<div class="border-t bg-surface-secondary px-4 py-2">
						<div class="max-h-60 overflow-auto">
							<ObjectViewer json={result} pureViewer />
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{:else}
	<Loader2 class="animate-spin" />
{/if}
