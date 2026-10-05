<script lang="ts">
	import { page } from '$app/state'
	import { Alert } from '$lib/components/common'
	import ServiceLogsInner from '$lib/components/ServiceLogsInner.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { devopsRole } from '$lib/stores'

	// `query` is what the global search links with; `searchTerm` is what the page writes back.
	let searchTerm = $state(
		page.url.searchParams.get('searchTerm') ?? page.url.searchParams.get('query') ?? ''
	)
</script>

{#snippet title()}
	<div class="flex items-center gap-2">
		<h1 class="text-2xl font-semibold text-emphasis whitespace-nowrap">Service logs</h1>
		<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/service_logs">
			Logs written by the servers, workers and indexers of this instance, grouped by host.
		</Tooltip>
	</div>
{/snippet}

<!-- `h-full`, not `h-screen`: the page header band sits above this box, so a viewport floor here
     would overhang the content box by the band's height. -->
<div class="flex flex-col w-full min-w-0 h-full max-h-full px-4">
	{#if $devopsRole == false}
		<div class="flex flex-col gap-2 pt-4">
			{@render title()}
			<Alert title="Service logs are only available to superadmins and devops users" type="warning">
				Ask one of them to look at the logs, or to grant you the devops role.
			</Alert>
		</div>
	{:else}
		<ServiceLogsInner bind:searchTerm {title} />
	{/if}
</div>
