<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { Alert } from './common'

	let ips: string[] | undefined = undefined

	WorkerService.listWorkers({ pingSince: 300 }).then((workers) => {
		ips = [
			...new Set(
				workers
					.filter((worker) => {
						return worker.ip != 'unretrievable IP' && worker.last_ping && worker.last_ping < 300
					})
					.map((worker) => worker.ip)
			)
		]
	})
</script>

{#if ips}
	<div class="mt-4"></div>
	<Alert size="xs" type="info" title="IPs to whitelist">
		<span class="text-tertiary">If necessary, the workers IPs to whitelist are:</span>
		{ips.join(', ')}
	</Alert>
{/if}
