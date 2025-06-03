<script lang="ts">
	import { Button } from '$lib/components/common'
	import { WandSparkles } from 'lucide-svelte'
	import { AIChatService } from './copilot/chat/AIChatManager.svelte'
	interface Props {
		label?: string
		initialInput?: string
		onClick?: () => void
	}

	const { label, initialInput, onClick: onClickProp }: Props = $props()

	export function onClick() {
		AIChatService.open = true
		if (initialInput) {
			AIChatService.initialInput = initialInput
		}
		onClickProp?.()
	}
</script>

<Button
	size="xs2"
	color="blue"
	buttonType="button"
	on:click={onClick}
	startIcon={{ icon: WandSparkles }}>{label}</Button
>
