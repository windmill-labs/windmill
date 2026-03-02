<script lang="ts">
	import { SettingsService } from '$lib/gen'

	let version = ''

	loadVersion()

	async function loadVersion() {
		version = await SettingsService.backendVersion()
	}
</script>

<div class="text-secondary text-xs">
	{version}
</div>
