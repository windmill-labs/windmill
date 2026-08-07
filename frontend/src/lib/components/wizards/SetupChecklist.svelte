<script lang="ts" module>
	export type SetupStepStatus = 'pending' | 'running' | 'done' | 'failed' | 'skipped'

	export type SetupStep = {
		title: string
		status: SetupStepStatus
		/** Shown when the row is expanded, and opened automatically when the step fails. */
		description?: string
	}

	/**
	 * A backend that only reports once it is done leaves every step blank while it works.
	 * Drive the list off that: the first unreported step is the one in flight.
	 */
	export function runningFrom(steps: SetupStep[], running: boolean): SetupStep[] {
		if (!running) return steps
		const next = steps.findIndex((s) => s.status === 'pending')
		return steps.map((s, i) => (i === next ? { ...s, status: 'running' } : s))
	}
</script>

<script lang="ts">
	import { Circle, CircleCheck, CircleX, ChevronDown, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import ResizeTransitionWrapper from '../common/ResizeTransitionWrapper.svelte'

	type Props = {
		steps: SetupStep[]
		class?: string
	}

	let { steps, class: className = '' }: Props = $props()

	let openedDescriptions: Record<number, true> = $state({})

	$effect(() => {
		for (let i = 0; i < steps.length; i++) {
			if (steps[i].status === 'failed') openedDescriptions[i] = true
		}
	})

	const titleClass: Record<SetupStepStatus, string> = {
		pending: 'text-hint/75',
		running: 'text-primary',
		done: 'text-green-600 dark:text-green-400',
		failed: 'text-red-400',
		skipped: 'text-hint/75'
	}
</script>

<div class={twMerge('flex flex-col gap-0.5', className)}>
	{#each steps as step, i}
		{@const descriptionOpened = openedDescriptions[i] ?? false}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="flex flex-col bg-surface rounded-md py-1 pr-2 cursor-pointer"
			role=""
			onclick={() => {
				if (!step.description) return
				if (descriptionOpened) delete openedDescriptions[i]
				else openedDescriptions[i] = true
			}}
		>
			<div class="flex gap-2">
				<span class="inline-flex w-4 h-5 shrink-0 justify-center items-center">
					{#if step.status === 'running'}
						<Loader2 size={16} class="inline animate-spin text-blue-500" />
					{:else if step.status === 'done'}
						<CircleCheck size={16} class="inline text-green-500" />
					{:else if step.status === 'failed'}
						<CircleX size={16} class="inline text-red-500" />
					{:else}
						<Circle size={16} class="inline text-hint/50" />
					{/if}
				</span>
				<div class="flex-1 my-0.5">
					<span
						class={twMerge(
							'text-xs font-medium flex justify-between items-center',
							titleClass[step.status]
						)}
					>
						{step.title}
						{#if step.description}
							<ChevronDown
								class={twMerge(
									'text-hint transition-transform',
									descriptionOpened ? 'rotate-180' : ''
								)}
								size={14}
							/>
						{/if}
					</span>
					<ResizeTransitionWrapper vertical class="text-2xs text-secondary">
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
