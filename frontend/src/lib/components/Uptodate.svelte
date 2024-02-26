<script lang="ts">
	import { SettingsService } from '$lib/gen'
	import { isCloudHosted } from '$lib/cloud'
	import Tooltip from './Tooltip.svelte'

	let uptodate: string | undefined = undefined
	let tooltipContent: string = "How to update?<br />- docker: `docker compose up -d`<br />- <a href='https://github.com/windmill-labs/windmill-helm-charts#install'>helm</a>"

	loadVersion()

	async function loadVersion() {
		try {
			const res = await SettingsService.backendUptodate()
			if (res != 'yes') {
				uptodate = res
				if (isCloudHosted()) {
					tooltipContent = "The cloud version is updated daily."
				}
			}
		} catch (e) {
			console.warn('Could not fetch latest version', e)
		}
	}
</script>

{#if uptodate}
	<span class="text-blue-400">{uptodate} &nbsp;<Tooltip>{@html tooltipContent}</Tooltip></span>
{/if}
