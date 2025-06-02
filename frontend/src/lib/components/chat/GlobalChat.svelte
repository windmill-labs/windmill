<script lang="ts">
	import { userStore, copilotInfo } from '$lib/stores'
	import AiChat from '../copilot/chat/AIChat.svelte'
	import { base } from '$lib/base'

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

<div class="relative flex flex-col h-full bg-surface z-20">
	<AiChat navigatorMode disabled={!hasCopilot} {disabledMessage} {suggestions} />
</div>
