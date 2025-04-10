<script context="module" lang="ts">
	export type CaptureInfo = {
		active: boolean
		hasPreprocessor: boolean
		canHavePreprocessor: boolean
		isFlow: boolean
		path: string
		connectionInfo: ConnectionInfo | undefined
		loading?: boolean
	}
</script>

<script lang="ts">
	import { slide } from 'svelte/transition'
	import AnimatedButton from '../common/button/AnimatedButton.svelte'
	import PulseButton from '../common/button/PulseButton.svelte'
	import Button from '../common/button/Button.svelte'
	import { CircleStop } from 'lucide-svelte'
	import ConnectionIndicator, {
		type ConnectionInfo
	} from '../common/alert/ConnectionIndicator.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import { createEventDispatcher, onDestroy, getContext } from 'svelte'
	import type { CaptureTriggerKind } from '$lib/gen'
	import CaptureIcon from './CaptureIcon.svelte'
	import Tooltip from '../Tooltip.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	export let disabled: boolean
	export let captureType: CaptureTriggerKind
	export let captureInfo: CaptureInfo
	export let captureTable: CaptureTable | undefined

	const dispatch = createEventDispatcher<{
		captureToggle: { disableOnly?: boolean }
		updateSchema: { payloadData: Record<string, any>; redirect: boolean }
	}>()

	const { showCaptureHint } = getContext<TriggerContext>('TriggerContext')

	onDestroy(() => {
		if (captureInfo.active) {
			dispatch('captureToggle', {
				// this on destroy can be called after capturing has already been stopped (aka after on destroy of the wrapper), make sure we do not start it again
				disableOnly: true
			})
		}
	})

	function handleUpdateSchema(e: any) {
		dispatch('updateSchema', {
			payloadData: e.detail.payloadData,
			redirect: e.detail.redirect
		})
	}

	let openingDuration = 400
	let pulseButton: PulseButton | undefined
	function updateShowCaptureHint(show: boolean | undefined) {
		if (show) {
			$showCaptureHint = false
			setTimeout(() => {
				pulseButton?.triggerPulse(1)
			}, 300)
		}
	}
	$: updateShowCaptureHint($showCaptureHint)
</script>

<div transition:slide={{ duration: openingDuration }} class="pb-12 overflow-hidden">
	<div class="border p-4 rounded-lg">
		<div class="flex flex-col gap-1 mb-4">
			<div class="flex flex-row items-center justify-start gap-1">
				<PulseButton bind:this={pulseButton} numberOfPulses={1} pulseDuration={1}>
					<AnimatedButton
						animate={captureInfo.active || captureInfo.loading}
						baseRadius="6px"
						wrapperClasses="ml-[-2px]"
					>
						<Button
							size="xs2"
							on:click={() => dispatch('captureToggle', {})}
							variant="border"
							{disabled}
							color="light"
							btnClasses={captureInfo.active ? 'text-blue-500 hover:text-blue-500' : ''}
						>
							<div class="flex flex-row items-center gap-1 w-28 justify-center">
								{#if captureInfo.active}
									<CircleStop size={14} />
								{:else}
									<CaptureIcon variant="redDot" size={14} />
								{/if}
								{captureInfo.active ? 'Stop' : 'Start capturing'}
							</div>
						</Button>
					</AnimatedButton>
				</PulseButton>

				{#if captureInfo.active}
					<ConnectionIndicator connectionInfo={captureInfo.connectionInfo} />
				{:else}
					<Tooltip>
						Start capturing to test your runnables with real data. Once active, all incoming
						payloads will be captured and displayed below, allowing you to test your runnables
						effectively.
					</Tooltip>
				{/if}
			</div>

			{#if disabled}
				<div class="text-sm font-normal text-red-600 dark:text-red-400" transition:slide>
					Enter a valid configuration to start capturing.
				</div>
			{/if}
		</div>

		{#if $$slots.default}
			<div class:opacity-50={disabled || !captureInfo.active} class="flex flex-col gap-4 mb-4">
				<slot />
			</div>
		{/if}

		<CaptureTable
			bind:this={captureTable}
			{captureType}
			hasPreprocessor={captureInfo.hasPreprocessor}
			canHavePreprocessor={captureInfo.canHavePreprocessor}
			isFlow={captureInfo.isFlow}
			path={captureInfo.path}
			canEdit={true}
			on:applyArgs
			on:updateSchema={handleUpdateSchema}
			on:addPreprocessor
			on:testWithArgs
			fullHeight={false}
			captureActiveIndicator={captureInfo.active}
		/>
	</div>
</div>
