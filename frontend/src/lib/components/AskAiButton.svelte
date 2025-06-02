<script lang="ts">
	import { Button } from '$lib/components/common'
	import { WandSparkles } from 'lucide-svelte'
	import { globalChatOpen, globalChatInitialInput } from '$lib/stores'

	interface Props {
		label?: string
		initialInput?: string
		onClick?: () => void
	}

	const { label, initialInput, onClick: onClickProp }: Props = $props()

	export function onClick() {
		globalChatOpen.set(true)
		if (initialInput) {
			globalChatInitialInput.set(initialInput)
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
