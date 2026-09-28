<script lang="ts">
	import { tick } from 'svelte'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import SessionsBetaBanner from '$lib/components/sessions/SessionsBetaBanner.svelte'
	import {
		dismissTiming,
		resetSessionsBetaBanner,
		sessionsBetaBanner
	} from '$lib/components/sessions/sessionsBetaBannerState.svelte'
	import AssistantSettingsHintTarget from '$lib/components/sessions/AssistantSettingsHintTarget.svelte'
	import { SlidersHorizontal } from 'lucide-svelte'

	// Playground for the sessions beta banner dismissal: the REAL banner next to a
	// mock composer whose settings button pulses. Dismissing here writes the same
	// localStorage key as the app, so the page clears it on load to start with the
	// banner shown — which also brings the banner back in the app.
	resetSessionsBetaBanner()

	let composerAtBottom = $state(false)
	let frame: HTMLDivElement | undefined = $state()

	async function replay() {
		resetSessionsBetaBanner()
		await tick()
		frame?.querySelector<HTMLButtonElement>('button[aria-label="Dismiss"]')?.click()
	}
</script>

<div class="p-8 max-w-4xl mx-auto flex flex-col gap-6">
	<div class="flex items-center justify-between">
		<h1 class="text-lg font-semibold text-emphasis">Beta banner dismiss playground</h1>
		<DarkModeToggle />
	</div>
	<p class="text-xs text-secondary">
		Click the banner's × or Replay. The banner disappears and the assistant settings button pulses
		once. Reset brings the banner back, here and in the app.
	</p>

	<div class="grid grid-cols-2 gap-x-8 gap-y-4 text-xs text-primary">
		<label class="flex flex-col gap-1">
			<span>Pulse duration: <b>{dismissTiming.pulseMs}ms</b></span>
			<input type="range" min="200" max="4000" step="100" bind:value={dismissTiming.pulseMs} />
		</label>
		<Toggle
			bind:checked={composerAtBottom}
			options={{ right: 'Composer above the banner (conversation layout)' }}
			size="xs"
		/>
	</div>

	<div class="flex items-center gap-2">
		<Button variant="accent" unifiedSize="sm" onclick={replay}>Replay</Button>
		<Button variant="default" unifiedSize="sm" onclick={resetSessionsBetaBanner}>Reset</Button>
		<span class="text-xs text-secondary">
			{sessionsBetaBanner.dismissed ? 'Dismissed' : 'Shown'}
		</span>
	</div>

	<div
		bind:this={frame}
		class="h-[420px] flex flex-col border border-border-light rounded-md bg-surface-secondary"
	>
		<div class="flex flex-col {composerAtBottom ? 'grow justify-end' : ''}">
			<div class="w-full max-w-3xl mx-auto px-6 pt-4 pb-1 flex flex-col gap-1">
				<div
					class="h-10 px-3 flex items-center rounded-md border border-border-light bg-surface text-xs text-hint"
				>
					Build a CRUD app for a customer table
				</div>
				<div class="flex items-center justify-end gap-2 text-xs text-secondary">
					<span>gpt-4o-mini</span>
					<AssistantSettingsHintTarget>
						<Button
							unifiedSize="2xs"
							variant="subtle"
							iconOnly
							startIcon={{ icon: SlidersHorizontal }}
							aria-label="Assistant settings"
						/>
					</AssistantSettingsHintTarget>
				</div>
			</div>
		</div>
		{#if !composerAtBottom}
			<div class="grow"></div>
		{/if}
		<SessionsBetaBanner variant="session" />
	</div>
</div>
