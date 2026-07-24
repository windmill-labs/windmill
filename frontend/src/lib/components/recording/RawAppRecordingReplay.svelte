<script lang="ts">
	/**
	 * Player for a raw-app session recording: walks the captured interactions one
	 * step at a time, rendering each step's DOM snapshot in a scripting-disabled
	 * iframe with the element the user acted on highlighted.
	 */
	import { Button } from '$lib/components/common'
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
	import {
		REC_TARGET_ATTR,
		withHighlightStyles,
		type RawAppInteractionKind
	} from './rawAppSnapshot'
	import type { RawAppRecording } from './types'

	let { recording }: { recording: RawAppRecording } = $props()

	/** 0 = the app as it was when recording started; 1..n = after step n-1 fired. */
	let stepIndex = $state(0)
	/** Within a step: the DOM the user acted on, then the DOM the app settled into. */
	let phase = $state<'before' | 'after'>('before')
	let playing = $state(false)
	let paneWidth = $state(0)
	let frameEl: HTMLIFrameElement | undefined = $state(undefined)

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
		step ? (phase === 'after' ? (step.after ?? step.before) : step.before) : 0
	)
	let html = $derived(frameIdx !== undefined ? recording.frames?.[frameIdx] : undefined)
	let srcdoc = $derived(html !== undefined ? withHighlightStyles(html) : undefined)

	/** Bring the highlighted element into view. The snapshot iframe is
	 * same-origin (srcdoc) but script-less, so the scroll is driven from here —
	 * on the frame's own window only, never `scrollIntoView` (which would also
	 * scroll this page). */
	function onFrameLoad() {
		const win = frameEl?.contentWindow
		const el = frameEl?.contentDocument?.querySelector(`[${REC_TARGET_ATTR}]`)
		if (!win || !el) return
		const rect = el.getBoundingClientRect()
		if (rect.top >= 0 && rect.bottom <= win.innerHeight) return
		win.scrollTo({ top: Math.max(0, win.scrollY + rect.top - win.innerHeight / 2), left: 0 })
	}

	let scale = $derived(
		recording.viewport?.width && paneWidth ? Math.min(1, paneWidth / recording.viewport.width) : 1
	)

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
						Recorded {new Date(recording.recorded_at).toLocaleString()} ·
						{fmtTime(recording.total_duration_ms)} ·
						{recording.viewport?.width}×{recording.viewport?.height}
					</span>
				{/snippet}
			</Tooltip>
			{#if recording.truncated}
				<span class="flex items-center gap-1 text-2xs text-yellow-600 dark:text-yellow-400">
					<TriangleAlert size={14} /> snapshots truncated
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
				<div class="flex rounded-md border overflow-hidden ml-1">
					{#each ['before', 'after'] as const as p (p)}
						<button
							class="px-2 py-1 text-2xs {phase === p
								? 'bg-surface-selected text-primary'
								: 'text-secondary hover:bg-surface-hover'}"
							onclick={() => {
								playing = false
								clearTimer()
								phase = p
							}}
						>
							{p === 'before' ? 'Interaction' : 'Result'}
						</button>
					{/each}
				</div>
			{/if}
		</div>
	</div>

	<div class="flex flex-1 min-h-0 gap-2">
		<div class="w-72 shrink-0 overflow-auto border rounded-md bg-surface-secondary">
			<button
				class="w-full text-left px-3 py-2 border-b border-l-2 flex items-center gap-2 {stepIndex === 0
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
				<!-- Snapshots are untrusted markup from a recording file: no
				     `allow-scripts`, so nothing in them executes. `allow-same-origin`
				     is what lets the player scroll the highlighted element into view. -->
				<iframe
					bind:this={frameEl}
					title="app-snapshot"
					{srcdoc}
					sandbox="allow-same-origin"
					onload={onFrameLoad}
					class="bg-white border-none block"
					style="width: {recording.viewport?.width || 1280}px; height: {recording.viewport
						?.height || 800}px; transform: scale({scale}); transform-origin: top left;"
				></iframe>
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
