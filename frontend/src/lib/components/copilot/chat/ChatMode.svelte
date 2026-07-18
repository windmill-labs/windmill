<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AIMode } from './AIChatManager.svelte'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()

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
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="subtle"
				endIcon={{ icon: ChevronDown }}
				title="Switch mode (Shift+Tab)"
			>
				{modeLabel(aiChatManager.mode)}
			</Button>
		{/snippet}
	</DropdownV2>
{:else}
	<Button unifiedSize="2xs" variant="subtle">
		{modeLabel(aiChatManager.mode)}
	</Button>
{/if}
