<script lang="ts">
	import { onMount, tick } from 'svelte'
	import { CheckCircle2, CircleHelp, Loader2, XCircle } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { UserQuestionChoice, UserQuestionDisplay } from './shared'

	interface Props {
		toolCallId: string
		userQuestion: UserQuestionDisplay
		disabled?: boolean
	}

	let { toolCallId, userQuestion, disabled = false }: Props = $props()

	let choiceButtons = $state<(HTMLButtonElement | undefined)[]>([])

	const selectedChoice = $derived(
		userQuestion.selectedChoiceId
			? userQuestion.choices.find((choice) => choice.id === userQuestion.selectedChoiceId)
			: undefined
	)
	const isComplete = $derived(Boolean(selectedChoice) || Boolean(userQuestion.canceled))
	const optionsDisabled = $derived(disabled || isComplete)

	onMount(() => {
		if (optionsDisabled || userQuestion.choices.length === 0) {
			return
		}

		void tick().then(() => {
			focusChoice(0)
		})
	})

	function focusChoice(index: number) {
		choiceButtons[index]?.focus()
	}

	function selectChoice(choice: UserQuestionChoice) {
		if (optionsDisabled) {
			return
		}
		aiChatManager.handleUserQuestionAnswer(toolCallId, choice)
	}

	function handleChoiceKeydown(event: KeyboardEvent, choice: UserQuestionChoice, index: number) {
		if (optionsDisabled) {
			return
		}

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

<div class="rounded-md border border-gray-200 bg-surface p-3 text-sm dark:border-gray-700">
	<div class="flex items-start gap-2">
		{#if userQuestion.canceled}
			<XCircle class="mt-0.5 h-4 w-4 shrink-0 text-red-500" />
		{:else if selectedChoice}
			<CheckCircle2 class="mt-0.5 h-4 w-4 shrink-0 text-green-500" />
		{:else}
			<CircleHelp class="mt-0.5 h-4 w-4 shrink-0 text-blue-500" />
		{/if}
		<p class="min-w-0 flex-1 whitespace-pre-wrap font-medium text-primary"
			>{userQuestion.question}</p
		>
	</div>

	<div class="mt-3 flex flex-col gap-2">
		{#each userQuestion.choices as choice, index (choice.id)}
			{@const isSelected = selectedChoice?.id === choice.id}
			<Button
				variant="default"
				unifiedSize="sm"
				bind:element={choiceButtons[index]}
				selected={isSelected}
				disabled={optionsDisabled}
				onClick={() => selectChoice(choice)}
				onkeydown={(event) => handleChoiceKeydown(event, choice, index)}
				startIcon={isSelected ? { icon: CheckCircle2 } : undefined}
				btnClasses="!h-auto min-h-[40px] !items-start !justify-start !px-3 !py-2 !text-left !whitespace-normal"
			>
				<span class="flex min-w-0 flex-col items-start gap-0.5">
					<span class="max-w-full break-words font-medium">{choice.label}</span>
					{#if choice.description}
						<span class="max-w-full break-words text-xs text-secondary">{choice.description}</span>
					{/if}
				</span>
			</Button>
		{/each}
	</div>

	{#if userQuestion.canceled}
		<div class="mt-2 text-xs text-secondary">Canceled</div>
	{:else if disabled}
		<div class="mt-2 flex items-center gap-1.5 text-xs text-secondary">
			<Loader2 class="h-3 w-3 animate-spin" />
			Waiting for selection
		</div>
	{/if}
</div>
