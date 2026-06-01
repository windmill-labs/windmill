<script lang="ts">
	import { onMount, tick } from 'svelte'
	import { CircleHelp, ArrowUp } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { UserQuestionDisplay } from './shared'

	interface Props {
		toolCallId: string
		userQuestion: UserQuestionDisplay
	}

	let { toolCallId, userQuestion }: Props = $props()

	let choiceButtons = $state<(HTMLButtonElement | undefined)[]>([])
	let customAnswerInput = $state<TextInput | undefined>()
	let customAnswer = $state('')
	let canSubmitCustomAnswer = $derived(customAnswer.trim().length > 0)

	// The custom-answer input is the last stop in the roving cursor, after all
	// choices, so arrow navigation can reach it from the keyboard.
	const customAnswerIndex = $derived(userQuestion.choices.length)
	const itemCount = $derived(userQuestion.choices.length + 1)
	// The item currently under the keyboard cursor. Mirrored onto the matching
	// choice's `selected` prop so the highlight comes from the Button itself.
	let activeIndex = $state(0)

	onMount(() => {
		void tick().then(() => {
			focusIndex(0)
		})
	})

	function focusIndex(index: number) {
		activeIndex = index
		if (index === customAnswerIndex) {
			customAnswerInput?.focus()
		} else {
			choiceButtons[index]?.focus()
		}
	}

	function selectChoice(choice: string) {
		aiChatManager.handleUserQuestionAnswer(toolCallId, choice)
	}

	function submitCustomAnswer() {
		const answer = customAnswer.trim()
		if (!answer) {
			return
		}

		aiChatManager.handleUserQuestionAnswer(toolCallId, answer)
	}

	function handleChoiceKeydown(event: KeyboardEvent, choice: string, index: number) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((index + 1) % itemCount)
			return
		}

		if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((index - 1 + itemCount) % itemCount)
			return
		}

		if (event.key === 'Enter') {
			event.preventDefault()
			event.stopPropagation()
			selectChoice(choice)
		}
	}

	function handleCustomAnswerKeydown(event: KeyboardEvent) {
		// Only ArrowUp/ArrowDown roam out of the input — Left/Right stay free for
		// moving the text caret within the answer.
		if (event.key === 'ArrowUp') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((customAnswerIndex - 1 + itemCount) % itemCount)
			return
		}

		if (event.key === 'ArrowDown') {
			event.preventDefault()
			event.stopPropagation()
			focusIndex((customAnswerIndex + 1) % itemCount)
			return
		}

		if (event.key === 'Enter') {
			event.preventDefault()
			event.stopPropagation()
			submitCustomAnswer()
		}
	}
</script>

<div
	class="rounded-md border border-border-light bg-surface p-3"
	data-chat-keyboard-scope="ask-user-question"
>
	<div class="flex items-start gap-2">
		<CircleHelp class="mt-0.5 h-4 w-4 shrink-0 text-accent" />
		<p class="min-w-0 flex-1 whitespace-pre-wrap text-xs font-semibold text-emphasis"
			>{userQuestion.question}</p
		>
	</div>

	<div class="mt-3 flex flex-col gap-2">
		{#each userQuestion.choices as choice, index (index)}
			<Button
				variant="default"
				unifiedSize="sm"
				selected={activeIndex === index}
				bind:element={choiceButtons[index]}
				onClick={() => selectChoice(choice)}
				onkeydown={(event) => handleChoiceKeydown(event, choice, index)}
				onfocus={() => (activeIndex = index)}
				btnClasses="h-auto min-h-7 justify-start whitespace-normal py-1.5 text-left focus-visible:!ring-0 focus-visible:!outline-none"
			>
				<span class="break-words">{choice}</span>
			</Button>
		{/each}

		<div class="flex min-w-0 gap-2 pt-1">
			<TextInput
				bind:this={customAnswerInput}
				bind:value={customAnswer}
				class="min-w-0 flex-1"
				size="sm"
				inputProps={{
					'aria-label': 'Custom answer',
					placeholder: 'Custom answer',
					onkeydown: handleCustomAnswerKeydown,
					onfocus: () => (activeIndex = customAnswerIndex)
				}}
			/>
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				title="Send"
				startIcon={{ icon: ArrowUp }}
				disabled={!canSubmitCustomAnswer}
				onClick={submitCustomAnswer}
				btnClasses="shrink-0"
			/>
		</div>
	</div>
</div>
