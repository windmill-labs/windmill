<script lang="ts">
	import { userStore, copilotInfo, globalChatOpen } from '$lib/stores'
	import AiChat from '../AIChat.svelte'
	import { base } from '$lib/base'
	import HideButton from '../../../apps/editor/settingsPanel/HideButton.svelte'

	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const hasCopilot = $derived($copilotInfo.enabled)
	const disabledMessage = $derived(
		hasCopilot
			? ''
			: isAdmin
				? `Enable Windmill AI in your [workspace settings](${base}/workspace_settings?tab=ai) to use this chat`
				: 'Ask an admin to enable Windmill AI in this workspace to use this chat'
	)

	const suggestions = [
		'Where can i see my latest runs?',
		'How do i trigger a script with a webhook endpoint?',
		'How can I connect to a database?',
		'How do I schedule a recurring job?'
	]
</script>

<div class="relative flex flex-col h-full bg-surface z-20 border-l border-border">
	<AiChat
		navigatorMode
		disabled={!hasCopilot}
		{disabledMessage}
		{suggestions}
		headerLeft={aiChatHeaderLeft}
	/>
</div>

{#snippet aiChatHeaderLeft()}
	<HideButton
		hidden={false}
		direction="right"
		panelName="AI"
		shortcut="L"
		size="md"
		on:click={() => {
			globalChatOpen.set(!$globalChatOpen)
		}}
	/>
{/snippet}
