<script lang="ts">
	import { onMount, tick } from 'svelte'
	import { CircleHelp } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { UserQuestionDisplay } from './shared'

	interface Props {
		toolCallId: string
		userQuestion: UserQuestionDisplay
	}

	let { toolCallId, userQuestion }: Props = $props()

	let choiceButtons = $state<(HTMLButtonElement | undefined)[]>([])

	onMount(() => {
		if (userQuestion.choices.length === 0) {
			return
		}

		void tick().then(() => {
			focusChoice(0)
		})
	})

	function focusChoice(index: number) {
		choiceButtons[index]?.focus()
	}

	function selectChoice(choice: string) {
		aiChatManager.handleUserQuestionAnswer(toolCallId, choice)
	}

	function handleChoiceKeydown(event: KeyboardEvent, choice: string, index: number) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
			event.preventDefault()
			event.stopPropagation()
			focusChoice((index + 1) % userQuestion.choices.length)
			return
		}

		if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
			event.preventDefault()
			event.stopPropagation()
			focusChoice((index - 1 + userQuestion.choices.length) % userQuestion.choices.length)
			return
		}

		if (event.key === 'Enter') {
			event.preventDefault()
			event.stopPropagation()
			selectChoice(choice)
		}
	}
</script>

<div
	class="rounded-md border border-gray-200 bg-surface p-3 text-sm dark:border-gray-700"
	data-chat-keyboard-scope="ask-user-question"
>
	<div class="flex items-start gap-2">
		<CircleHelp class="mt-0.5 h-4 w-4 shrink-0 text-blue-500" />
		<p class="min-w-0 flex-1 whitespace-pre-wrap text-xs font-medium text-primary"
			>{userQuestion.question}</p
		>
	</div>

	<div class="mt-3 flex flex-col gap-2">
		{#each userQuestion.choices as choice, index (index)}
			<Button
				variant="default"
				unifiedSize="sm"
				bind:element={choiceButtons[index]}
				onClick={() => selectChoice(choice)}
				onkeydown={(event) => handleChoiceKeydown(event, choice, index)}
				btnClasses="!h-auto min-h-[40px] !items-start !justify-start !px-3 !py-2 !text-left !whitespace-normal"
			>
				<span class="flex min-w-0 flex-col items-start gap-0.5">
					<span class="max-w-full break-words text-2xs font-medium">{choice}</span>
				</span>
			</Button>
		{/each}
	</div>
</div>
