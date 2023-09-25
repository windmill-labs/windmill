<script lang="ts">
	import { SettingsService } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'

	let uptodate: string | undefined = undefined

	loadVersion()

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
</script>

{#if uptodate}
	<span class="text-blue-400"
		>{uptodate} &nbsp;<Tooltip>
			How to update?<br />
			- docker: `docker compose up -d`<br />-
			<a href="https://github.com/windmill-labs/windmill-helm-charts#install">helm</a>
		</Tooltip>
	</span>
{/if}
