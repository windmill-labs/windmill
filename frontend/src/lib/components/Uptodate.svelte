<script lang="ts">
	import { SettingsService } from '$lib/gen'
	import { isCloudHosted } from '$lib/cloud'
	import Tooltip from './Tooltip.svelte'

	let uptodate: string | undefined = undefined

	async function loadVersion() {
		try {
			const res = await SettingsService.backendUptodate()
			if (res != 'yes') {
				uptodate = res
			}
		} catch (e) {
			console.warn('Could not fetch latest version', e)
		}
	}

	loadVersion()
</script>

{#if uptodate}
	<span class="text-blue-400">
		{uptodate} &nbsp;
		<Tooltip>
			{#if isCloudHosted()}
				The cloud version is updated daily.
			{:else}
				How to update?<br />
				- docker: `docker compose up -d`<br />
				- <a href="https://github.com/windmill-labs/windmill-helm-charts#install">helm</a>
			{/if}
		</Tooltip>
	</span>
{/if}
