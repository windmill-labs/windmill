<script lang="ts">
	import { ZapIcon, ZapOffIcon } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { codeCompletionLoading, copilotInfo, codeCompletionSessionEnabled } from '$lib/stores'
	import Popover from '../Popover.svelte'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'

	const SETTING_NAME = 'codeCompletionSessionEnabled'
	function loadSetting() {
		$codeCompletionSessionEnabled = (getLocalSetting(SETTING_NAME) ?? 'true') == 'true'
	}

	function storeSetting() {
		$codeCompletionSessionEnabled = !$codeCompletionSessionEnabled
		storeLocalSetting(SETTING_NAME, $codeCompletionSessionEnabled.toString())
	}

	loadSetting()
</script>

{#if $copilotInfo.exists_openai_resource_path && $copilotInfo.code_completion_enabled}
	<Popover>
		<svelte:fragment slot="text"
			>{$codeCompletionSessionEnabled ? 'Disable' : 'Enable'} code completion (applies only to you)</svelte:fragment
		>
		<Button
			color="light"
			loading={$codeCompletionLoading}
			startIcon={$codeCompletionLoading
				? undefined
				: {
						icon: $codeCompletionSessionEnabled ? ZapIcon : ZapOffIcon
				  }}
			on:click={() => {
				storeSetting()
			}}
		/>
	</Popover>
{:else}
	<Popover>
		<svelte:fragment slot="text"
			>Code completion is disabled in the workspace settings</svelte:fragment
		>
		<Button
			color="light"
			startIcon={{
				icon: ZapOffIcon
			}}
			disabled
		/>
	</Popover>
{/if}
