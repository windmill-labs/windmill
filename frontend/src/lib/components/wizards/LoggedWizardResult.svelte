<script module lang="ts">
	export function firstEmptyStepIsError<Step extends { status?: LoggedWizardStatus }>(
		steps: Step[],
		error: string | undefined
	): (Step & { status: LoggedWizardStatus })[] {
		let convertedSteps = [...steps]
		let alreadyFoundEmpty = false
		for (let step of convertedSteps) {
			if (!step.status) {
				if (!alreadyFoundEmpty) {
					alreadyFoundEmpty = true
					step.status = error !== undefined ? 'FAIL' : 'SKIP'
				} else {
					step.status = 'SKIP'
				}
			}
		}
		return convertedSteps as any
	}
</script>

<script lang="ts">
	import type { LoggedWizardStatus } from '$lib/gen'
	import { CircleCheck, Circle, CircleX, ChevronDown } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import ResizeTransitionWrapper from '../common/ResizeTransitionWrapper.svelte'

	type Props = {
		steps: { title?: string; status: LoggedWizardStatus; description?: string }[]
		class?: string
	}

	let { steps, class: className = '' }: Props = $props()

	let openedDescriptions: Record<number, true> = $state({})

	$effect(() => {
		for (let i = 0; i < steps.length; i++) {
			let step = steps[i]
			if (step.status == 'FAIL') {
				openedDescriptions[i] = true
			}
		}
	})
</script>

<div class={twMerge('flex flex-col gap-2', className)}>
	{#each steps as step, i}
		{@const descriptionOpened = openedDescriptions[i] ?? false}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="flex flex-col bg-surface rounded-md py-1.5 pr-2 cursor-pointer"
			role=""
			onclick={() => {
				if (step.description) {
					if (descriptionOpened) delete openedDescriptions[i]
					else openedDescriptions[i] = true
				}
			}}
		>
			<div class="flex gap-3">
				<span class="inline-flex w-5 h-10 shrink-0 justify-center items-center">
					{#if step.status == 'SKIP'}
						<Circle size={20} class="inline text-hint/50" />
					{:else if step.status == 'FAIL'}
						<CircleX size={20} class="inline text-red-500" />
					{:else if step.status == 'OK'}
						<CircleCheck size={20} class="inline text-green-500" />
					{/if}
				</span>
				<div class="flex-1 my-2">
					<span
						class={twMerge(
							'font-medium flex justify-between items-center',
							{
								SKIP: 'text-hint/75',
								FAIL: 'text-red-400',
								OK: 'text-green-600 dark:text-green-400'
							}[step.status]
						)}
					>
						{i + 1}. {step.title}
						{#if step.description}
							<ChevronDown
								class={twMerge(
									'text-hint transition-transform',
									descriptionOpened ? 'rotate-180' : ''
								)}
								size={16}
							/>
						{/if}
					</span>
					<ResizeTransitionWrapper vertical class="text-xs text-secondary">
						{#if descriptionOpened}
							<div
								class="whitespace-pre-wrap cursor-default mt-1.5"
								onclick={(e) => e.stopPropagation()}
							>
								{step.description}
							</div>
						{/if}
					</ResizeTransitionWrapper>
				</div>
			</div>
		</div>
	{/each}
</div>
