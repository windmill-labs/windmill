<script lang="ts">
	import { ExternalLink, ZapIcon, ZapOffIcon } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { copilotInfo, metadataCompletionEnabled } from '$lib/stores'
	import Popover from '../Popover.svelte'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'

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
			>{$metadataCompletionEnabled ? 'Disable' : 'Enable'} metadata completion (applies only to you)</svelte:fragment
		>
		<Button
			color="light"
			startIcon={{
				icon: $metadataCompletionEnabled ? ZapIcon : ZapOffIcon
			}}
			on:click={() => {
				storeSetting()
			}}
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
			startIcon={{
				icon: ZapOffIcon
			}}
			disabled
		/>
	</Popover>
{/if}
