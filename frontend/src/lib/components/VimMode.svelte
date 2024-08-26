<script lang="ts">
	import { VIM_MODE_SETTING_NAME, vimMode } from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import Toggle from './Toggle.svelte'

	function storeSetting() {
		storeLocalSetting(VIM_MODE_SETTING_NAME, $vimMode.toString())
	}
</script>

<Toggle
	size="xs"
	bind:checked={$vimMode}
	on:change={() => {
		storeSetting()
	}}
	options={{ right: 'vim mode' }}
/>
