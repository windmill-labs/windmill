<script lang="ts">
	import { aiChatInstanceStore } from '$lib/stores'
	import { AIChatService } from '../AIChatManager.svelte'
	import AiChat from '../AIChat.svelte'
	import HideButton from '../../../apps/editor/settingsPanel/HideButton.svelte'

	let aiChatInstance: AiChat | undefined = undefined

	$effect(() => {
		if (aiChatInstance) {
			aiChatInstanceStore.set(aiChatInstance)
		}
		return () => {
			aiChatInstanceStore.set(undefined)
		}
	})
</script>

<div class="relative flex flex-col h-full bg-surface z-20 border-l border-border">
	<AiChat headerLeft={aiChatHeaderLeft} />
</div>

{#snippet aiChatHeaderLeft()}
	<HideButton
		hidden={false}
		direction="right"
		panelName="AI"
		shortcut="L"
		size="md"
		on:click={() => {
			AIChatService.open = !AIChatService.open
		}}
	/>
{/snippet}
