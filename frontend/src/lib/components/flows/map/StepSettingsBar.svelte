<script lang="ts" module>
	import type { OnSelectedIteration } from '$lib/components/graph/graphBuilder.svelte'

	/** A retried step's attempts in a run: failed attempts then the last, as the node state lists them. */
	export type RetryAttempts = {
		jobs: string[]
		success: (boolean | undefined)[]
		selected: number
		moduleId: string
		onSelectedIteration: OnSelectedIteration
	}
</script>

<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { msToCompactDuration, type Item } from '$lib/utils'
	import AttemptPicker from './AttemptPicker.svelte'
	import type { StepSettingKey, StepSettingView } from '../flowStepSettings'

	interface Props {
		/** The step's configured settings, in canonical order. */
		settings: StepSettingView[]
		/** Failed attempts so far. */
		retries?: number
		attempts?: RetryAttempts
		/** In a run, while its concurrency limit holds the step back. */
		held?: { since?: string; nextCheck?: Date | string }
		/** Makes each setting's row open that setting in the step's settings panel. */
		onOpenSetting?: (key: StepSettingKey) => void
		onOpenChange?: (open: boolean) => void
	}

	let {
		settings,
		retries = undefined,
		attempts = undefined,
		held = undefined,
		onOpenSetting,
		onOpenChange
	}: Props = $props()

	let open = $state(false)

	// Wider than 4 icons, the bar reaches the "+" insert button centred under the node in the editor.
	const MAX_ICONS = 4

	// Retries lead, out of the canonical order, so their count and attempt picker are never
	// under the "+N".
	let ordered = $derived([
		...settings.filter((s) => s.key === 'retries'),
		...settings.filter((s) => s.key !== 'retries')
	])
	let shown = $derived(ordered.slice(0, MAX_ICONS))
	let hiddenCount = $derived(ordered.length - shown.length)

	let pastAttempt = $derived(
		!!attempts && attempts.selected >= 0 && attempts.selected < attempts.jobs.length - 1
	)

	// Runs so far: the failed attempts and the last one. `retries` alone counts only failures.
	let runs = $derived(attempts?.jobs.length ?? (retries ? retries + 1 : undefined))

	let retrySetting = $derived(settings.find((s) => s.key === 'retries'))
	// Next to the runs, a bare "3 retries" reads as what happened.
	let policy = $derived(
		retrySetting?.summary.state === 'configured' && retrySetting.summary.text
			? `Up to ${retrySetting.summary.text}`
			: (retrySetting?.summary.text ?? '')
	)
	let pickable = $derived(attempts && attempts.jobs.length > 1 ? attempts : undefined)
	let pickerProps = $derived(
		pickable && {
			policy,
			attempts: pickable.jobs.map((_, index) => ({
				index,
				success: pickable.success[index],
				selected: index === pickable.selected
			})),
			onPick: (index: number) =>
				pickable.onSelectedIteration({
					moduleId: pickable.moduleId,
					index,
					id: pickable.jobs[index],
					manuallySet: true
				})
		}
	)
	// With nothing else to list, the menu is the attempt list itself, one hover fewer.
	let attemptsOnly = $derived(!!pickerProps && settings.length === 1)

	// Read as the menu opens: the held time it shows does not tick.
	let heldDetail = $derived(
		held?.since && open
			? ` · held ${msToCompactDuration(Math.max(Date.now() - new Date(held.since).getTime(), 0), 0)}`
			: held
				? ' · held'
				: undefined
	)

	let items: Item[] = $derived(
		ordered.map((s) => {
			const action = onOpenSetting && (() => onOpenSetting(s.key))
			if (s.key === 'concurrency' && heldDetail)
				return {
					displayName: s.label,
					icon: s.icon,
					iconColor: '#f97316',
					detail: s.summary.text + heldDetail,
					action
				}
			if (s.key !== 'retries')
				return { displayName: s.label, icon: s.icon, detail: s.summary.text, action }
			if (pickerProps) {
				return {
					displayName: s.label,
					icon: s.icon,
					extra: runCounter,
					chevronClass: 'text-orange-500',
					customSubmenu: AttemptPicker,
					customSubmenuProps: pickerProps
				}
			}
			return { displayName: s.label, icon: s.icon, detail: policy, action }
		})
	)
</script>

{#snippet runCounter()}
	<span class="ml-auto shrink-0 pl-4 text-2xs text-orange-500">{runs} attempts</span>
{/snippet}

<div transition:fade|local={{ duration: 200 }}>
	<DropdownV2
		{items}
		placement="bottom-end"
		usePointerDownOutside
		fixedHeight={false}
		customWidth={320}
		customMenu={attemptsOnly}
		bind:open={
			() => open,
			(v) => {
				open = v
				onOpenChange?.(v)
			}
		}
	>
		{#snippet menu({ item, close })}
			{#if pickerProps}
				<div
					class="bg-surface-tertiary dark:border origin-top-right rounded-lg shadow-lg focus:outline-none"
				>
					<AttemptPicker {...pickerProps} meltItem={item} {close} showCount />
				</div>
			{/if}
		{/snippet}
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				variant="default"
				unifiedSize="2xs"
				wrapperClasses="bg-surface rounded-md"
				btnClasses="px-2"
				title={held ? 'Step settings: held by its concurrency limit' : 'Step settings'}
			>
				<span class="flex flex-row items-center gap-1.5">
					{#each shown as s (s.key)}
						{@const Icon = s.icon}
						<span
							class="flex flex-row items-center gap-0.5 {(s.key === 'retries' && retries) ||
							(s.key === 'concurrency' && held)
								? 'text-orange-500'
								: ''}"
						>
							{#if s.key === 'retries' && pastAttempt && attempts}
								<span class="text-2xs">{attempts.selected + 1}/{attempts.jobs.length}</span>
							{:else if s.key === 'retries' && retries && runs}
								<span class="text-2xs">{runs}</span>
							{/if}
							<Icon size={12} />
						</span>
					{/each}
					{#if hiddenCount > 0}
						<span class="text-2xs text-secondary">+{hiddenCount}</span>
					{/if}
				</span>
			</Button>
		{/snippet}
	</DropdownV2>
</div>
