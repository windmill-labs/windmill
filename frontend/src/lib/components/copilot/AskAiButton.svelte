<script lang="ts">
	import { Button } from '$lib/components/common'
	import { WandSparkles } from 'lucide-svelte'
	import { aiChatManager } from './chat/AIChatManager.svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	import { prefersSessionHandoff } from './chat/global/gate'
	import { startSessionWithPrompt } from '$lib/components/sessions/sessionSwitch.svelte'
	import { userStore } from '$lib/stores'
	interface Props {
		label?: string
		initialInput?: string
		onClick?: () => void
	}

	const { label, initialInput, onClick: onClickProp }: Props = $props()

	export function onClick() {
		// No item to preview here — this carries a question, not a target — so the
		// hand-off opens a bare session with the text pre-filled in the composer.
		if (prefersSessionHandoff($userStore?.operator)) {
			onClickProp?.()
			void startSessionWithPrompt(initialInput ?? '')
			return
		}
		aiChatManager.openChat()
		if (initialInput) {
			aiChatManager.askAi(initialInput, {
				withCode: false,
				withDiff: false
			})
		}
		onClickProp?.()
	}
</script>

<Button
	iconOnly={!label}
	startIcon={{
		icon: WandSparkles
	}}
	unifiedSize="md"
	btnClasses={AIBtnClasses('default')}
	on:click={onClick}
>
	{label}
</Button>
