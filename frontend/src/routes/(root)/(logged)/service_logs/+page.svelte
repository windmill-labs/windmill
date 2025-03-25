<script lang="ts">
	import { page } from '$app/stores'
	import { Alert } from '$lib/components/common'
	import Popover from '$lib/components/Popover.svelte'
	import ServiceLogsInner from '$lib/components/ServiceLogsInner.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { devopsRole } from '$lib/stores'
	import { Search, AlertTriangle } from 'lucide-svelte'

	let searchTerm = $page.url.searchParams.get('query') ?? ''
	let queryParseErrors: string[] | undefined = undefined
</script>

<div class="flex flex-col w-full h-screen max-h-screen max-w-screen px-2">
	<div class="px-2">
		<div class="flex items-center space-x-2 flex-row justify-between">
			<div class="flex flex-row flex-wrap justify-between py-2 my-4 px-4 gap-1 items-center">
				<h1 class="!text-2xl font-semibold leading-6 tracking-tight">Service logs</h1>
				<Tooltip>Explore and search Windmill service logs from within Windmill!</Tooltip>
			</div>
		</div>
	</div>

	{#if !$devopsRole}
		<Alert title="Service logs are only available to superadmins" type="warning">
			Service logs are only available to superadmins (or devops)
		</Alert>
	{:else}
		<div class="m-1 px-2 flex flex-row gap-1 items-center border-2 rounded-lg">
			<Search size="16" />
			<!-- svelte-ignore a11y_autofocus -->
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
						Some of your search terms have been ignored because one or more parse errors:<br /><br
						/>
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
</div>

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
