<script lang="ts" module>
	export type SetupStepStatus = 'pending' | 'running' | 'done' | 'failed' | 'skipped'

	export type SetupStep = {
		title: string
		status: SetupStepStatus
		/** Shown when the row is expanded, and opened automatically when the step fails. */
		description?: string
		/** The checks this step is made of, when the caller knows them. Always visible: they
		 *  are the step's progress, not detail to go looking for. */
		substeps?: SetupStep[]
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
	import Self from './SetupChecklist.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResizeTransitionWrapper from '../common/ResizeTransitionWrapper.svelte'

	type Props = {
		steps: SetupStep[]
		class?: string
		/** Applied to each step's substep block, for a caller whose substeps are a long
		 *  list rather than a handful of checks and need their own scroll. */
		substepsClass?: string
	}

	let { steps, class: className = '', substepsClass = '' }: Props = $props()

	/**
	 * Only the steps the user has actually toggled. A failed step opens itself, so recording
	 * the open state instead would need something to force it open on every update -- and
	 * every progress update would then reopen a description the user had just closed.
	 */
	let userToggled: Record<number, boolean> = $state({})

	const descriptionOpen = (i: number, status: SetupStepStatus) =>
		userToggled[i] ?? status === 'failed'

	function toggleDescription(i: number, status: SetupStepStatus) {
		userToggled[i] = !descriptionOpen(i, status)
	}

	const titleRowClass = (status: SetupStepStatus) =>
		twMerge('text-xs font-medium flex justify-between items-center', titleClass[status])

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
		{@const descriptionOpened = descriptionOpen(i, step.status)}
		<div class="flex flex-col bg-surface rounded-md py-1 pr-2">
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
					<!-- The title is the whole interactive surface, so a step without a description
					stays inert rather than offering a focus stop that does nothing. -->
					{#if step.description}
						<button
							type="button"
							class={twMerge(titleRowClass(step.status), 'w-full text-left cursor-pointer')}
							onclick={() => toggleDescription(i, step.status)}
						>
							{step.title}
							<ChevronDown
								class={twMerge(
									'text-hint transition-transform',
									descriptionOpened ? 'rotate-180' : ''
								)}
								size={14}
							/>
						</button>
					{:else}
						<span class={titleRowClass(step.status)}>{step.title}</span>
					{/if}
					<ResizeTransitionWrapper vertical class="text-2xs text-secondary">
						{#if descriptionOpened}
							<div class="whitespace-pre-wrap mt-1.5">
								{step.description}
							</div>
						{/if}
					</ResizeTransitionWrapper>
				</div>
			</div>
			{#if step.substeps?.length}
				<div class={twMerge('ml-6', substepsClass)}>
					<Self steps={step.substeps} {substepsClass} />
				</div>
			{/if}
		</div>
	{/each}
</div>
