<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let license: string

	let valid = false
	$: license && checkLicenseKey(license)

	async function checkLicenseKey(key: string) {
		try {
			const { LicenseManager } = await import('ag-grid-enterprise')

			let details = LicenseManager.getLicenseDetails(key)
			valid = details.valid
		} catch (e) {
			console.error(e)
		}
	}
</script>

<div class="p-2">
	<span class="text-xs font-semibold">AgGrid EE License Key</span>
	<input type="text" bind:value={license} placeholder="AgGrid Enterprise" />

	{#if valid}
		<Badge color="green">Valid</Badge>
	{:else}
		<Badge color="red">Invalid</Badge>
	{/if}
</div>
