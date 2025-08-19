<script lang="ts">
	import { RELATIVE_LINE_NUMBERS_SETTING_NAME, relativeLineNumbers } from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import Toggle from './Toggle.svelte'

	function storeSetting() {
		storeLocalSetting(RELATIVE_LINE_NUMBERS_SETTING_NAME, $relativeLineNumbers.toString())
	}
</script>

<Toggle
	size="xs"
	bind:checked={$relativeLineNumbers}
	on:change={() => {
		storeSetting()
	}}
	options={{ right: 'relative line numbers' }}
/>