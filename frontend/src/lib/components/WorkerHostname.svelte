<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { ExternalLink, Loader2 } from 'lucide-svelte'

	type Props = {
		worker: string
		minTs?: string
	}

	let { worker, minTs }: Props = $props()

	let instances = $state(undefined) as
		| Record<string, { hostname: string; worker_group: string }>
		| undefined
	async function loadWorkers() {
		const res = await WorkerService.listWorkers({})
		instances = res.reduce(
			(acc, worker) => {
				acc[worker.worker] = { hostname: worker.worker_instance, worker_group: worker.worker_group }
				return acc
			},
			{} as Record<string, { hostname: string; worker_group: string }>
		)
	}

	loadWorkers()
</script>

{#if instances == undefined}
	<Loader2 />
{:else if instances[worker]}
	host: {instances[worker].hostname}
	<br />
	worker group: {instances[worker].worker_group}
	<br />
	<a
		href="/service_logs?mode=worker&workerGroup={instances[worker]
			.worker_group}&hostname={instances[worker].hostname}{minTs
			? `&minTs=${encodeURIComponent(minTs)}`
			: ''}"
		target="_blank"
	>
		service logs <ExternalLink size={14} class="inline-block" />
	</a>
{/if}
