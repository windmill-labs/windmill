<script lang="ts">
	import { SettingsService } from '$lib/gen'

	let version = ''

	loadVersion()

	async function loadVersion() {
		version = await SettingsService.backendVersion()
	}
</script>

{version}
