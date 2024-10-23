<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ServiceLogsInner from '$lib/components/ServiceLogsInner.svelte'
	import { superadmin } from '$lib/stores'
	import { Search } from 'lucide-svelte'

	let searchTerm = ''
</script>

<CenteredPage class="max-w-full">
	<PageHeader title="Service Logs" tooltip="Search windmill service logs" />

	{#if !$superadmin}
		<Alert title="Service logs are only available to superadmins" type="warning">
			Service logs are only available to superadmins
		</Alert>
	{:else}
		<div class="px-2 flex flex-row gap-1 items-center border-2 rounded-lg">
			<Search size="16" />
			<input
				id="quickSearchInput"
				type="text"
				class="quick-search-input !bg-surface"
				bind:value={searchTerm}
				autocomplete="off"
			/>
		</div>

		<ServiceLogsInner {searchTerm} />
	{/if}
</CenteredPage>

<style>
	.quick-search-input {
		outline: none;
		border: none !important;
		box-shadow: none !important;
	}

	.quick-search-input:focus-visible {
		outline: none !important;
	}
</style>
