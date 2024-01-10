<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let license: string

	let valid = false
	$: license && checkLicenseKey(license)

	async function checkLicenseKey(key: string) {
		try {
			const { AgCharts } = await import('ag-charts-enterprise')

			// @ts-ignore
			valid = AgCharts.licenseKey
		} catch (e) {
			console.error(e)
		}
	}
</script>

<div class="p-2">
	<span class="text-xs font-semibold">AgCharts EE License Key</span>
	<input type="text" bind:value={license} placeholder="AgCharts Enterprise" />

	{#if valid}
		<Badge color="green">Valid</Badge>
	{:else}
		<Badge color="red">Invalid</Badge>
	{/if}
</div>
