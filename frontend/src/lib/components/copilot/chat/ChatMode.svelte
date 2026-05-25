<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import { CHAT_BAR_PILL, CHAT_BAR_PILL_STATIC } from './chatBarStyles'

	const modeLabel = (mode: AIMode) => mode.charAt(0).toUpperCase() + mode.slice(1) + ' mode'

	let allowedModeList = $derived(
		Object.values(AIMode).filter((mode) => aiChatManager.allowedModes[mode])
	)
	let hasMultiple = $derived(allowedModeList.length > 1)
</script>

{#if hasMultiple}
	<DropdownV2
		items={() =>
			allowedModeList.map((mode) => ({
				displayName: modeLabel(mode),
				selected: aiChatManager.mode === mode,
				action: () => aiChatManager.changeMode(mode)
			}))}
		placement="bottom-start"
		fixedHeight={false}
		customWidth={170}
		class="min-w-0"
	>
		{#snippet buttonReplacement()}
			<div class={CHAT_BAR_PILL}>
				<span class="truncate">{modeLabel(aiChatManager.mode)}</span>
				<ChevronDown size={14} class="shrink-0" />
			</div>
		{/snippet}
	</DropdownV2>
{:else}
	<div class={CHAT_BAR_PILL_STATIC}>
		<span class="truncate">{modeLabel(aiChatManager.mode)}</span>
	</div>
{/if}
