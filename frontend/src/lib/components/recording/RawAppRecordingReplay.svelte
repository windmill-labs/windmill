<script lang="ts">
	/**
	 * Player for a raw-app session recording: walks the captured interactions one
	 * step at a time, rendering each step's DOM snapshot in a scripting-disabled
	 * iframe with the element the user acted on highlighted.
	 */
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import {
		ChevronLeft,
		ChevronRight,
		CircleDot,
		InfoIcon,
		Keyboard,
		ListChecks,
		MousePointerClick,
		Navigation,
		Pause,
		Play,
		SendHorizonal,
		TextCursorInput,
		ToggleLeft,
		TriangleAlert
	} from 'lucide-svelte'
	import { onDestroy } from 'svelte'
	import { withHighlightStyles, type RawAppInteractionKind } from './rawAppSnapshot'
	import type { RawAppRecording } from './types'

	let { recording }: { recording: RawAppRecording } = $props()

	/** 0 = the app as it was when recording started; 1..n = after step n-1 fired. */
	let stepIndex = $state(0)
	/** Within a step: the DOM the user acted on, then the DOM the app settled into. */
	let phase = $state<'before' | 'after'>('before')
	let playing = $state(false)
	let paneWidth = $state(0)
	let stepList: HTMLElement | undefined = $state(undefined)

	const KIND_ICONS: Record<RawAppInteractionKind, any> = {
		click: MousePointerClick,
		fill: TextCursorInput,
		select: ListChecks,
		toggle: ToggleLeft,
		submit: SendHorizonal,
		key: Keyboard,
		navigate: Navigation
	}

	let steps = $derived(recording.steps ?? [])
	let step = $derived(stepIndex > 0 ? steps[stepIndex - 1] : undefined)
	let frameIdx = $derived(
		step ? (phase === 'after' ? (step.after ?? step.before) : (step.before ?? step.after)) : 0
	)
	let html = $derived(frameIdx !== undefined ? recording.frames?.[frameIdx] : undefined)
	let srcdoc = $derived(html !== undefined ? withHighlightStyles(html) : undefined)

	// Belt and braces with the loader's validation: these numbers land in a `style`
	// string, and the player also renders recordings handed to it directly.
	function size(v: unknown, fallback: number): number {
		return typeof v === 'number' && Number.isFinite(v) && v > 0 && v <= 20000 ? Math.round(v) : fallback
	}
	let frameWidth = $derived(size(recording.viewport?.width, 1280))
	let frameHeight = $derived(size(recording.viewport?.height, 800))

	let scale = $derived(paneWidth ? Math.min(1, paneWidth / frameWidth) : 1)

	/** Left offset (%) of each step's checkpoint on the timeline: its recorded
	 * time, then a spreading pass so bursts of fast interactions stay clickable. */
	let marks = $derived.by(() => {
		const total = recording.total_duration_ms || steps[steps.length - 1]?.t || 1
		const min = steps.length > 1 ? Math.min(6, 92 / (steps.length - 1)) : 0
		let last = -Infinity
		return steps.map((s) => {
			const at = Math.max(0, Math.min(96, (s.t / total) * 92 + 4))
			const pos = Math.max(at, last + min)
			last = pos
			return Math.min(99, pos)
		})
	})

	// Keep the playing step visible: during playback the list scrolls past the
	// viewport within a few steps.
	$effect(() => {
		const row = stepList?.querySelector(`[data-step="${stepIndex}"]`)
		row?.scrollIntoView({ block: 'nearest' })
	})

	function goto(index: number, at: 'before' | 'after' = 'before') {
		stepIndex = Math.max(0, Math.min(steps.length, index))
		phase = stepIndex === 0 ? 'before' : at
	}

	let timer: ReturnType<typeof setTimeout> | undefined = undefined

	function clearTimer() {
		if (timer) clearTimeout(timer)
		timer = undefined
	}

	/** Auto-advance: hold on the interaction, then on its outcome for as long as
	 * the user actually paused before the next one (clamped so a long think-time
	 * doesn't stall the playback). */
	function schedule() {
		clearTimer()
		if (!playing) return
		if (stepIndex === 0) {
			timer = setTimeout(() => {
				goto(1)
				schedule()
			}, 800)
			return
		}
		if (phase === 'before') {
			timer = setTimeout(() => {
				phase = 'after'
				schedule()
			}, 900)
			return
		}
		if (stepIndex >= steps.length) {
			playing = false
			return
		}
		const gap = Math.min(2000, Math.max(400, steps[stepIndex].t - steps[stepIndex - 1].t))
		timer = setTimeout(() => {
			goto(stepIndex + 1)
			schedule()
		}, gap)
	}

	function togglePlay() {
		if (playing) {
			playing = false
			clearTimer()
			return
		}
		if (stepIndex >= steps.length && phase === 'after') goto(0)
		playing = true
		schedule()
	}

	function step_(index: number) {
		playing = false
		clearTimer()
		goto(index)
	}

	onDestroy(clearTimer)

	function fmtTime(ms: number): string {
		return `${(ms / 1000).toFixed(1)}s`
	}
</script>

<div class="flex flex-col h-full min-h-0 gap-2">
	<div class="flex items-center justify-between gap-2 shrink-0">
		<div class="flex items-center gap-2 min-w-0">
			<h2 class="text-lg font-semibold text-emphasis truncate">
				{recording.app_path || 'Untitled app'}
			</h2>
			<span class="text-xs text-secondary px-2 py-0.5 bg-surface-secondary rounded shrink-0">
				{steps.length} step{steps.length === 1 ? '' : 's'}
			</span>
			<Tooltip placement="bottom">
				<InfoIcon size={16} class="text-tertiary" />
				{#snippet text()}
					<span class="text-2xs">
						{recording.workspace ? `${recording.workspace} · ` : ''}Recorded {new Date(
							recording.recorded_at
						).toLocaleString()} ·
						{fmtTime(recording.total_duration_ms)} ·
						{recording.viewport?.width}×{recording.viewport?.height}
					</span>
				{/snippet}
			</Tooltip>
			{#if recording.truncated}
				<span class="flex items-center gap-1 text-2xs text-yellow-600 dark:text-yellow-400">
					<TriangleAlert size={14} /> recording truncated
				</span>
			{/if}
		</div>
		<div class="flex items-center gap-1 shrink-0">
			<Button
				variant="border"
				size="xs"
				iconOnly
				startIcon={{ icon: ChevronLeft }}
				disabled={stepIndex === 0}
				onclick={() => step_(stepIndex - 1)}
			/>
			<Button
				variant="border"
				size="xs"
				startIcon={{ icon: playing ? Pause : Play }}
				disabled={steps.length === 0}
				onclick={togglePlay}
			>
				{playing ? 'Pause' : 'Play'}
			</Button>
			<Button
				variant="border"
				size="xs"
				iconOnly
				startIcon={{ icon: ChevronRight }}
				disabled={stepIndex >= steps.length}
				onclick={() => step_(stepIndex + 1)}
			/>
			{#if step}
				<div class="ml-1">
					<ToggleButtonGroup
						selected={phase}
						on:selected={(e) => {
							// The group also fires when playback advances the phase itself;
							// only a real click (a phase we are not already on) pauses.
							if (e.detail === phase) return
							playing = false
							clearTimer()
							phase = e.detail
						}}
					>
						{#snippet children({ item })}
							<ToggleButton size="sm" value="before" label="Interaction" {item} />
							<ToggleButton size="sm" value="after" label="Result" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
			{/if}
		</div>
	</div>

	<!-- Timeline: one checkpoint per interaction, placed at the time it happened.
	     Clicking a checkpoint jumps the player to that step. -->
	<div class="shrink-0 px-2 pt-1 pb-4">
		<div class="relative h-6">
			<div class="absolute inset-x-0 top-3 h-px bg-surface-selected"></div>
			<div
				class="absolute left-0 top-3 h-px bg-blue-500 transition-all"
				style="width: {stepIndex === 0 ? 0 : (marks[stepIndex - 1] ?? 0)}%"
			></div>
			<button
				class="absolute top-1.5 -translate-x-1/2 size-3 rounded-full border-2 {stepIndex === 0
					? 'bg-blue-500 border-blue-500'
					: 'bg-surface border-surface-selected hover:border-blue-400'}"
				style="left: 0%"
				title="Initial state"
				aria-label="Initial state"
				onclick={() => step_(0)}
			></button>
			{#each steps as s, i (i)}
				{@const active = stepIndex === i + 1}
				<button
					class="absolute -translate-x-1/2 rounded-full border-2 {active
						? 'top-1 size-4 bg-blue-500 border-blue-500'
						: 'top-1.5 size-3 border-surface-selected hover:border-blue-400 ' +
							(stepIndex > i + 1 ? 'bg-blue-200 dark:bg-blue-900' : 'bg-surface')}"
					style="left: {marks[i]}%"
					title="{i + 1}. {s.label} (+{fmtTime(s.t)})"
					aria-label="Step {i + 1}: {s.label}"
					onclick={() => step_(i + 1)}
				></button>
			{/each}
		</div>
		<div class="flex justify-between text-2xs text-tertiary">
			<span>0s</span>
			<span>{fmtTime(recording.total_duration_ms)}</span>
		</div>
	</div>

	<div class="flex flex-1 min-h-0 gap-2">
		<div
			bind:this={stepList}
			class="w-72 shrink-0 overflow-auto border rounded-md bg-surface-secondary"
		>
			<button
				data-step="0"
				class="w-full text-left px-3 py-2 border-b border-l-2 flex items-center gap-2 {stepIndex ===
				0
					? 'bg-surface border-l-blue-500'
					: 'border-l-transparent hover:bg-surface-hover'}"
				onclick={() => step_(0)}
			>
				<CircleDot size={14} class="text-tertiary shrink-0" />
				<span class="text-xs text-primary">Initial state</span>
			</button>
			{#each steps as s, i (i)}
				{@const Icon = KIND_ICONS[s.kind] ?? MousePointerClick}
				<button
					data-step={i + 1}
					class="w-full text-left px-3 py-2 border-b border-l-2 flex items-start gap-2 {stepIndex ===
					i + 1
						? 'bg-surface border-l-blue-500'
						: 'border-l-transparent hover:bg-surface-hover'}"
					onclick={() => step_(i + 1)}
				>
					<span class="text-2xs text-tertiary w-5 shrink-0 pt-0.5">{i + 1}</span>
					<Icon size={14} class="text-tertiary shrink-0 mt-0.5" />
					<span class="min-w-0 flex-1">
						<span class="block text-xs text-primary break-words">{s.label}</span>
						<span class="block text-2xs text-tertiary">+{fmtTime(s.t)}</span>
					</span>
				</button>
			{/each}
		</div>

		<div
			class="flex-1 min-w-0 overflow-auto border rounded-md bg-surface"
			bind:clientWidth={paneWidth}
		>
			{#if srcdoc !== undefined}
				<!-- A recording is untrusted markup, and `?src=` loads one from any URL,
				     so the snapshot renders under the empty sandbox: no scripting, and an
				     opaque origin that can't reach the viewer's Windmill session. -->
				<!-- The wrapper carries the scaled-down box; the iframe keeps the recorded
				     viewport size so the app lays out exactly as it did. -->
				<div style="width: {frameWidth * scale}px; height: {frameHeight * scale}px;">
					<iframe
						title="app-snapshot"
						{srcdoc}
						sandbox=""
						class="bg-white border-none block"
						style="width: {frameWidth}px; height: {frameHeight}px; transform: scale({scale}); transform-origin: top left;"
					></iframe>
				</div>
			{:else}
				<div class="h-full flex items-center justify-center text-sm text-secondary p-4 text-center">
					No snapshot was captured for this step.
				</div>
			{/if}
		</div>
	</div>

	{#if step}
		<div class="shrink-0 text-xs text-secondary border rounded-md px-3 py-2 bg-surface-secondary">
			<span class="font-semibold text-primary">Step {stepIndex}/{steps.length}:</span>
			{step.label}
			{#if step.selector}
				<span class="text-2xs text-tertiary font-mono ml-2">{step.selector}</span>
			{/if}
		</div>
	{/if}
</div>
