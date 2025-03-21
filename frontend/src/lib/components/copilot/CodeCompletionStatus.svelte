<script lang="ts">
	import {
		codeCompletionSessionEnabled,
		copilotInfo,
		CODE_COMPLETION_SETTING_NAME
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import Toggle from '../Toggle.svelte'

	function storeSetting() {
		storeLocalSetting(CODE_COMPLETION_SETTING_NAME, $codeCompletionSessionEnabled.toString())
	}
</script>

{#if $copilotInfo.enabled && $copilotInfo.codeCompletionModel}
	<Toggle
		size="xs"
		bind:checked={$codeCompletionSessionEnabled}
		on:change={() => {
			storeSetting()
		}}
		options={{ right: 'AI code completion' }}
	/>
{:else}
	<Toggle
		disabled
		size="xs"
		checked={false}
		on:change={() => {
			storeSetting()
		}}
		options={{ right: 'AI code completion (disabled in workspace settings)' }}
	/>
{/if}
