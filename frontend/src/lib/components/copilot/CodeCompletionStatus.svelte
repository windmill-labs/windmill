<script lang="ts">
	import { getAiChatManager } from '$lib/components/copilot/chat/aiChatManagerContext'
	import { codeCompletionSessionEnabled, CODE_COMPLETION_SETTING_NAME } from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import Toggle from '../Toggle.svelte'

	const aiChat = getAiChatManager()
	const aiInfo = $derived(aiChat.aiInfo)

	function storeSetting() {
		storeLocalSetting(CODE_COMPLETION_SETTING_NAME, $codeCompletionSessionEnabled.toString())
	}
</script>

{#if aiInfo?.enabled && aiInfo.codeCompletionModel}
	<Toggle
		size="xs"
		bind:checked={$codeCompletionSessionEnabled}
		on:change={() => {
			storeSetting()
		}}
		options={{ right: 'AI code completion' }}
	/>
{:else if aiInfo}
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
