<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { Alert } from './common'

	let ips: string[] | undefined = $state(undefined)

	// Sentinels the backend stores when a worker's external IP is unknown: 'NO IP' while the
	// lookup is pending or after it failed, 'unretrievable IP' from workers predating that.
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
	<div class="mt-4"></div>
	<Alert size="xs" type="info" title="IPs to whitelist">
		<span class="text-primary">If necessary, the workers IPs to whitelist are:</span>
		{ips.join(', ')}
	</Alert>
{/if}
