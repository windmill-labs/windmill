<script lang="ts">
	import { WorkerService } from '$lib/gen'
	import { Alert } from './common'

	let ips: string[] | undefined = undefined

	WorkerService.listWorkers({}).then((workers) => {
		ips = [
			...new Set(
				workers
					.filter((worker) => {
						const date = new Date().getTime() - 300 * 60
						const ping_at = new Date(worker.ping_at).getTime()
						return ping_at > date
					})
					.map((worker) => worker.ip)
			)
		]
	})
</script>

{#if ips}
	<div class="mt-2" />
	<Alert size="xs" type="info" title="IPs to whitelist">
		<span class="text-gray-600">If necessary, the workers IPs to whitelist are:</span>
		{ips.join(', ')}
	</Alert>
{/if}
