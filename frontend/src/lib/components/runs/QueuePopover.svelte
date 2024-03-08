<script lang="ts">
	import { JobService, QueuedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import { displayDate } from '$lib/utils'

	let jobs: QueuedJob[] | undefined = undefined
	export let allWorkspaces: boolean = false

	getQueuedJobs()
	async function getQueuedJobs() {
		jobs = await JobService.listQueue({
			workspace: $workspaceStore ?? '',
			scheduledForBeforeNow: true,
			suspended: false,
			running: false,
			allWorkspaces
		})
	}
</script>

{#if jobs == undefined}
	<Skeleton
		layout={[
			[2, 2],
			[2, 2]
		]}
	/>
{:else}
	<div class="flex flex-col gap-2 text-sm">
		{#if jobs.length > 100}
			<div class="text-secondary text-xs">Only showing the first 100 jobs</div>
		{/if}

		{#each jobs.slice(0, 100) as job}
			<div class="flex">
				<a
					target="_blank"
					href={`/run/${job.id}?workspace=${job.workspace_id}`}
					class="flex flex-row gap-2 items-center font-mono mr-8"
					>{job.id} <ExternalLink size={10} />
				</a>
				<div class="w-32">{displayDate(job.created_at)}</div>
				<div class="text-2xs text-tertiary">tag: {job.tag}</div>
			</div>
		{/each}
	</div>
{/if}
