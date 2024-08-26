<script lang="ts">
	import { FORMAT_ON_SAVE_SETTING_NAME, formatOnSave } from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import Toggle from './Toggle.svelte'

	function storeSetting() {
		storeLocalSetting(FORMAT_ON_SAVE_SETTING_NAME, $formatOnSave.toString())
	}
</script>

<Toggle
	size="xs"
	bind:checked={$formatOnSave}
	on:change={() => {
		storeSetting()
	}}
	options={{ right: 'auto-formatting' }}
/>
