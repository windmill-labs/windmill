<script lang="ts">
	import { Button } from '$lib/components/common'
	import { WandSparkles } from 'lucide-svelte'
	import { aiChatManager } from './chat/AIChatManager.svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	interface Props {
		label?: string
		initialInput?: string
		onClick?: () => void
	}

	const { label, initialInput, onClick: onClickProp }: Props = $props()

	export function onClick() {
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
