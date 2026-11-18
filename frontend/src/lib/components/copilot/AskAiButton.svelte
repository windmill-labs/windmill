<script lang="ts">
	import { Button } from '$lib/components/common'
	import { WandSparkles } from 'lucide-svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	import { startSessionWithPrompt } from '$lib/components/sessions/sessionSwitch.svelte'
	interface Props {
		label?: string
		initialInput?: string
		onClick?: () => void
	}

	const { label, initialInput, onClick: onClickProp }: Props = $props()

	export function onClick() {
		onClickProp?.()
		// No item to preview here — this carries a question, not a target — so it opens a bare
		// session on the question alone, sent on arrival: it is what the user already typed.
		void startSessionWithPrompt(initialInput ?? '', { autoSend: true })
	}
</script>

<Button
	iconOnly={!label}
	startIcon={{
		icon: WandSparkles
	}}
	unifiedSize="md"
	btnClasses={AIBtnClasses('default')}
	title="Ask this in a new AI session"
	on:click={onClick}
>
	{label}
</Button>
