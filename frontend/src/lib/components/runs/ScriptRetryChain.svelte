<script lang="ts">
	import { JobService, type Job } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'

	let { job }: { job: Job } = $props()

	type AttemptColor = 'green' | 'red' | 'blue' | 'gray'
	type Attempt = { id: string; color: AttemptColor; current: boolean }
	type HandlerKind = 'failure' | 'recovery' | 'success'
	type HandlerLink = { id: string; kind: HandlerKind; color: AttemptColor }
	type Info = {
		retries: Attempt[]
		handlers: HandlerLink[]
		self?: { kind: HandlerKind; schedulePath?: string; handledJob?: string }
	}

	const HANDLER_LABEL: Record<HandlerKind, string> = {
		failure: 'Failure handler',
		recovery: 'Recovery handler',
		success: 'Success handler'
	}

	function statusColor(j: Job | undefined): AttemptColor {
		if (!j) return 'gray'
		if (j.type === 'CompletedJob') return j.success ? 'green' : 'red'
		if (j.type === 'QueuedJob' && j.running) return 'blue'
		return 'gray'
	}

	// A schedule completion handler is recognizable by its synthetic `created_by`.
	// `on_recovery` and `on_success` share it, so the recovery-only `error_started_at`
	// arg disambiguates them (absent => plain success). Needs the full job (the list
	// endpoint omits args), so callers pass a job fetched via getJob or the page job.
	function handlerKind(j: Job | undefined): HandlerKind | undefined {
		if (j?.created_by === 'schedule_error_handler') return 'failure'
		if (j?.created_by === 'schedule_recovery_handler') {
			return j?.args && 'error_started_at' in j.args ? 'recovery' : 'success'
		}
		return undefined
	}

	const info = resource(
		() => ({ job, ws: job?.workspace_id ?? $workspaceStore }),
		async ({ job, ws }): Promise<Info> => {
			if (!ws || !job) return { retries: [], handlers: [] }

			// The current job IS a handler: describe it rather than look for a chain.
			const self = handlerKind(job)
			if (self) {
				return {
					retries: [],
					handlers: [],
					self: {
						kind: self,
						schedulePath: (job.args as any)?.schedule_path,
						handledJob: job.parent_job
					}
				}
			}

			// Native script retries are real `Script` jobs linked by `parent_job` to the
			// first attempt (the chain root, itself a script). Flow steps also carry
			// `parent_job`, but their parent is a flow — a non-script root means flow step.
			if (job.job_kind !== 'script') return { retries: [], handlers: [] }
			const root = job.parent_job ?? job.id
			const rootJob = job.id === root ? job : await JobService.getJob({ workspace: ws, id: root })
			if (rootJob?.job_kind !== 'script') return { retries: [], handlers: [] }

			// No kind filter: retry attempts are scripts (kept via `is_retry` below),
			// but schedule handlers can be flows (flow/... handler paths), so both
			// kinds must be fetched for the handler row to find flow handlers.
			const children = await JobService.listJobs({
				workspace: ws,
				parentJob: root
			})
			// Retry attempts carry the explicit `is_retry` marker; other children
			// (WAC v2 inline children, flow/script handlers) do not, so only real
			// retries are kept here.
			const retryJobs = (children ?? [])
				.filter((c) => c.is_retry === true)
				.sort((a, b) => (a.created_at ?? '').localeCompare(b.created_at ?? ''))
			const chain: (Job | undefined)[] = [rootJob, ...retryJobs]
			const retries: Attempt[] =
				retryJobs.length === 0
					? []
					: chain.map((j) => ({
							id: j?.id ?? root,
							color: statusColor(j),
							current: j?.id === job.id
						}))

			// Schedule handlers fire on the terminal attempt, so they are its children.
			const terminalId = chain[chain.length - 1]?.id ?? root
			const handlerChildren =
				terminalId === root
					? children
					: await JobService.listJobs({ workspace: ws, parentJob: terminalId })
			// `created_by` flags a handler; fetch the full job to read `args` (the list
			// endpoint omits it) so recovery and success can be told apart.
			const handlers: HandlerLink[] = (
				await Promise.all(
					(handlerChildren ?? [])
						.filter(
							(c) =>
								c.created_by === 'schedule_error_handler' ||
								c.created_by === 'schedule_recovery_handler'
						)
						.map(async (c) => {
							const full = await JobService.getJob({ workspace: ws, id: c.id })
							const kind = handlerKind(full)
							return kind ? { id: c.id, kind, color: statusColor(full) } : undefined
						})
				)
			).filter((h): h is HandlerLink => h !== undefined)

			return { retries, handlers }
		}
	)

	const self = $derived(info.current?.self)
	const retries = $derived(info.current?.retries ?? [])
	const handlers = $derived(info.current?.handlers ?? [])
	const ws = $derived(job?.workspace_id ?? $workspaceStore)
</script>

{#if self}
	<div class="max-w-7xl mx-auto w-full px-4 mt-12">
		<div class="text-xs text-emphasis font-semibold mb-1">{HANDLER_LABEL[self.kind]}</div>
		<div class="flex flex-row flex-wrap gap-2 items-center">
			{#if self.handledJob}
				<Button
					size="xs"
					color="gray"
					variant="border"
					onclick={() => goto(`/run/${self?.handledJob}?workspace=${ws}`)}
				>
					Handled run
				</Button>
			{/if}
			{#if self.schedulePath}
				<span class="text-xs text-secondary">
					for schedule <span class="font-mono">{self.schedulePath}</span>
				</span>
			{/if}
		</div>
	</div>
{:else}
	{#if retries.length > 1}
		<div class="max-w-7xl mx-auto w-full px-4 mt-12">
			<div class="text-xs text-emphasis font-semibold mb-1">Retries ({retries.length - 1})</div>
			<div class="flex flex-row flex-wrap gap-2">
				{#each retries as attempt, i (attempt.id)}
					<Button
						size="xs"
						color={attempt.color}
						variant={attempt.current ? 'contained' : 'border'}
						onclick={() => goto(`/run/${attempt.id}?workspace=${ws}`)}
					>
						{i === 0 ? 'Original' : `Attempt ${i}`}
					</Button>
				{/each}
			</div>
		</div>
	{/if}
	{#if handlers.length > 0}
		<div class="max-w-7xl mx-auto w-full px-4 mt-4">
			<div class="text-xs text-emphasis font-semibold mb-1">Handlers</div>
			<div class="flex flex-row flex-wrap gap-2">
				{#each handlers as handler (handler.id)}
					<Button
						size="xs"
						color={handler.color}
						variant="border"
						onclick={() => goto(`/run/${handler.id}?workspace=${ws}`)}
					>
						{HANDLER_LABEL[handler.kind]}
					</Button>
				{/each}
			</div>
		</div>
	{/if}
{/if}
