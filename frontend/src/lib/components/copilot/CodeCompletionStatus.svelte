<script lang="ts">
	import {
		codeCompletionSessionEnabled,
		copilotInfo,
		FORMAT_ON_SAVE_SETTING_NAME
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import Toggle from '../Toggle.svelte'

	function storeSetting() {
		storeLocalSetting(FORMAT_ON_SAVE_SETTING_NAME, $codeCompletionSessionEnabled.toString())
	}
</script>

{#if $copilotInfo.exists_openai_resource_path && $copilotInfo.code_completion_enabled}
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
