<script lang="ts">
	import { page } from '$app/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ServiceLogsInner from '$lib/components/ServiceLogsInner.svelte'
	import { superadmin } from '$lib/stores'
	import { Search, AlertTriangle } from 'lucide-svelte'


	let searchTerm = $page.url.searchParams.get('query') ?? ''
	let queryParseErrors: string[] | undefined = undefined
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
				autofocus
			/>
			{#if searchTerm !== '' && queryParseErrors && queryParseErrors.length > 0}
				<Popover notClickable placement="bottom-start">
					<AlertTriangle size={16} class="text-yellow-500" />
					<svelte:fragment slot="text">
						Some of your search terms have been ignored because one or more parse errors:<br
						/><br />
						<ul>
							{#each queryParseErrors as msg}
								<li>- {msg}</li>
							{/each}
						</ul>
					</svelte:fragment>
				</Popover>
			{/if}
		</div>

		<ServiceLogsInner {searchTerm} bind:queryParseErrors />
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
