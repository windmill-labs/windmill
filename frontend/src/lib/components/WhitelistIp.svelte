<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { Alert } from './common'

	let ips: string[] | undefined = $state(undefined)

	// Sentinels the backend stores when a worker has no external IP to report: 'NO IP' while the
	// lookup is still in flight, 'unretrievable IP' once it has failed.
	const UNKNOWN_IPS = ['NO IP', 'unretrievable IP']

	WorkerService.listWorkers({ pingSince: 300 }).then((workers) => {
		ips = [
			...new Set(
				workers
					.filter((worker) => {
						return !UNKNOWN_IPS.includes(worker.ip) && worker.last_ping && worker.last_ping < 300
					})
					.map((worker) => worker.ip)
			)
		]
	})
</script>

{#if ips}
	<Alert size="xs" type="info" title="IPs to whitelist">
		<span class="text-primary">If necessary, the workers IPs to whitelist are:</span>
		{ips.join(', ')}
	</Alert>
{/if}
