<script lang="ts">
	import { JobService, type Job } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'

	let { job }: { job: Job } = $props()

	type AttemptColor = 'green' | 'red' | 'blue' | 'gray'
	type Attempt = { id: string; color: AttemptColor; current: boolean }

	function statusColor(j: Job | undefined): AttemptColor {
		if (!j) return 'gray'
		if (j.type === 'CompletedJob') return j.success ? 'green' : 'red'
		if (j.type === 'QueuedJob' && j.running) return 'blue'
		return 'gray'
	}

	// Native script retries are real `Script` jobs linked by `parent_job` to the
	// first attempt (the chain root, itself a script). Flow steps also carry
	// `parent_job`, but their parent is a flow — so a non-script root means this
	// is a flow step, not a retry, and we render nothing.
	const attempts = resource(
		() => ({ job, ws: job?.workspace_id ?? $workspaceStore }),
		async ({ job, ws }): Promise<Attempt[]> => {
			const root = job?.parent_job ?? job?.id
			if (!ws || !root || job?.job_kind !== 'script') {
				return []
			}
			const rootJob = job?.id === root ? job : await JobService.getJob({ workspace: ws, id: root })
			if (rootJob?.job_kind !== 'script') {
				return []
			}
			const children = await JobService.listJobs({
				workspace: ws,
				parentJob: root,
				jobKinds: 'script'
			})
			// Keep only genuine retry attempts: re-runs of the SAME script. Schedule
			// completion handlers (on_failure/on_recovery/on_success) are also `script`
			// children of the root but run a different script, so exclude them.
			const retries = (children ?? [])
				.filter((c) => c.script_hash === rootJob.script_hash)
				.sort((a, b) => (a.created_at ?? '').localeCompare(b.created_at ?? ''))
			if (retries.length === 0) {
				return []
			}
			const chain: (Job | undefined)[] = [rootJob, ...retries]
			return chain.map((j) => ({
				id: j?.id ?? root,
				color: statusColor(j),
				current: j?.id === job?.id
			}))
		}
	)
</script>

{#if (attempts.current ?? []).length > 1}
	<div class="max-w-7xl mx-auto w-full px-4 mt-12">
		<div class="text-xs text-emphasis font-semibold mb-1">
			Retries ({(attempts.current ?? []).length - 1})
		</div>
		<div class="flex flex-row flex-wrap gap-2">
			{#each attempts.current ?? [] as attempt, i (attempt.id)}
				<Button
					size="xs"
					color={attempt.color}
					variant={attempt.current ? 'contained' : 'border'}
					onclick={() =>
						goto(`/run/${attempt.id}?workspace=${job?.workspace_id ?? $workspaceStore}`)}
				>
					{i === 0 ? 'Original' : `Attempt ${i}`}
				</Button>
			{/each}
		</div>
	</div>
{/if}
