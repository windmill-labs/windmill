<script lang="ts">
	import { SparklesIcon } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { codeCompletionLoading, copilotInfo, codeCompletionSessionEnabled } from '$lib/stores'
	import Popover from '../Popover.svelte'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import SparklesOffIcon from '../icons/SparklesOffIcon.svelte'

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
			>Code completion: {$codeCompletionSessionEnabled ? 'enabled' : 'disabled'} (applies only to you)</svelte:fragment
		>
		<Button
			color="light"
			size="xs"
			loading={$codeCompletionLoading}
			startIcon={$codeCompletionLoading
				? undefined
				: {
						icon: $codeCompletionSessionEnabled ? SparklesIcon : SparklesOffIcon,
						classes: $codeCompletionSessionEnabled
							? 'text-violet-800 dark:text-violet-400'
							: 'text-violet-800/50 dark:text-violet-400/50'
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
				icon: SparklesOffIcon
			}}
			btnClasses="text-violet-800/50 dark:text-violet-400/50"
		/>
	</Popover>
{/if}
