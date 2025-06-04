<script lang="ts">
	import { aiChatManager } from '../AIChatManager.svelte'
	import AiChat from '../AIChat.svelte'
	import HideButton from '../../../apps/editor/settingsPanel/HideButton.svelte'
</script>

<div class="relative flex flex-col h-full bg-surface z-20">
	<AiChat headerLeft={aiChatHeaderLeft} />
</div>

{#snippet aiChatHeaderLeft()}
	<HideButton
		hidden={false}
		direction="right"
		panelName="AI"
		shortcut="L"
		size="md"
		on:click={() => aiChatManager.toggleOpen()}
	/>
{/snippet}
