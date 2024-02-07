<script lang="ts">
	import { ExternalLink, Sparkles } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { copilotInfo, metadataCompletionEnabled } from '$lib/stores'
	import Popover from '../Popover.svelte'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import SparklesOffIcon from '../icons/SparklesOffIcon.svelte'

	const SETTING_NAME = 'metadataCompletionEnabled'
	function loadSetting() {
		$metadataCompletionEnabled = (getLocalSetting(SETTING_NAME) ?? 'true') == 'true'
	}

	function storeSetting() {
		$metadataCompletionEnabled = !$metadataCompletionEnabled
		storeLocalSetting(SETTING_NAME, $metadataCompletionEnabled.toString())
	}

	loadSetting()
</script>

{#if $copilotInfo.exists_openai_resource_path}
	<Popover>
		<svelte:fragment slot="text"
			>Metadata completion: {$metadataCompletionEnabled ? 'enabled' : 'disabled'} (applies only to you)</svelte:fragment
		>
		<Button
			size="xs"
			color="light"
			startIcon={{
				icon: $metadataCompletionEnabled ? Sparkles : SparklesOffIcon,
				classes: $metadataCompletionEnabled
					? 'text-violet-800 dark:text-violet-400'
					: 'text-violet-800/50 dark:text-violet-400/50'
			}}
			on:click={() => {
				storeSetting()
			}}
			btnClasses="p-1.5"
		/>
	</Popover>
{:else}
	<Popover>
		<svelte:fragment slot="text">
			Enable Windmill AI in the <a
				href="/workspace_settings?tab=openai"
				target="_blank"
				class="inline-flex flex-row items-center gap-1"
			>
				workspace settings <ExternalLink size={16} />
			</a>
		</svelte:fragment>
		<Button
			color="light"
			size="xs"
			class="text-violet-800/50 dark:text-violet-400/50"
			btnClasses="p-1.5"
			startIcon={{
				icon: SparklesOffIcon
			}}
		/>
	</Popover>
{/if}
