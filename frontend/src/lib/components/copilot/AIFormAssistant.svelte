<script lang="ts">
	import { Button } from '$lib/components/common'
	import { WandSparkles, Pencil } from 'lucide-svelte'
	import { aiChatManager } from './chat/AIChatManager.svelte'

	interface Props {
		onEditInstructions: () => void
		instructions: string
		runnableType: 'script' | 'flow'
	}

	const { onEditInstructions, instructions, runnableType }: Props = $props()

	async function fillFormWithAI() {
		aiChatManager.openChat()
		aiChatManager.askAi(`Analyze the ${runnableType} form on this page and fill the inputs for me`)
	}
</script>

<div class="my-3 p-3 bg-surface-secondary rounded-md">
	<div class="flex flex-row justify-between items-center">
		<div class="flex flex-col gap-1">
			<h3 class="text-sm font-medium">Fill the inputs with AI</h3>
			<p class="text-sm text-tertiary">
				{instructions
					? 'Instructions: ' + instructions
					: 'No AI instructions provided. Click edit to add guidance for AI form filling.'}
			</p>
		</div>
		<Button
			color="light"
			size="xs2"
			startIcon={{
				icon: Pencil
			}}
			iconOnly
			on:click={onEditInstructions}
		/>
	</div>
	<div class="flex justify-end mt-2">
		<Button
			size="xs2"
			startIcon={{
				icon: WandSparkles
			}}
			on:click={fillFormWithAI}
		>
			Fill with AI
		</Button>
	</div>
</div>
