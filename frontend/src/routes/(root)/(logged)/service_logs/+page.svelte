<script lang="ts">
	import { page } from '$app/state'
	import { Alert } from '$lib/components/common'
	import ServiceLogsInner from '$lib/components/ServiceLogsInner.svelte'
	import { devopsRole } from '$lib/stores'

	// `query` is what the global search links with; `searchTerm` is what the page writes back.
	let searchTerm = $state(
		page.url.searchParams.get('searchTerm') ?? page.url.searchParams.get('query') ?? ''
	)
</script>

<!-- `h-full`, not `h-screen`: the page header band sits above this box, so a viewport floor here
     would overhang the content box by the band's height. -->
<div class="flex flex-col w-full min-w-0 h-full max-h-full px-4">
	{#if $devopsRole == false}
		<div class="flex flex-col gap-2 pt-4">
			<Alert title="Service logs are only available to superadmins and devops users" type="warning">
				Ask one of them to look at the logs, or to grant you the devops role.
			</Alert>
		</div>
	{:else}
		<ServiceLogsInner bind:searchTerm />
	{/if}
</div>
