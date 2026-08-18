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

	// The label stays short ("Ask AI") for the search bar's inline row; the hover
	// text is where "new AI session" fits.
	const handsOffToSession = $derived(prefersSessionHandoff($userStore?.operator))

	export function onClick() {
		// No item to preview here — this carries a question, not a target — so the
		// hand-off opens a bare session on the question alone.
		if (handsOffToSession) {
			onClickProp?.()
			// Sent on arrival, matching the legacy path below (askAi sends straight
			// away): the text is the question the user already typed.
			void startSessionWithPrompt(initialInput ?? '', { autoSend: true })
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
	title={handsOffToSession ? 'Ask this in a new AI session' : 'Ask this in the AI chat'}
	on:click={onClick}
>
	{label}
</Button>
