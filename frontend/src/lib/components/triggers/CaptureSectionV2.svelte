<script context="module" lang="ts">
	export type CaptureInfo = {
		active: boolean
		hasPreprocessor: boolean
		canHavePreprocessor: boolean
		isFlow: boolean
		path: string
		connectionInfo: ConnectionInfo | undefined
	}
</script>

<script lang="ts">
	import { slide } from 'svelte/transition'
	import AnimatedButton from '../common/button/AnimatedButton.svelte'
	import PulseButton from '../common/button/PulseButton.svelte'
	import Button from '../common/button/Button.svelte'
	import { CircleStop, History, AlertCircle } from 'lucide-svelte'
	import ConnectionIndicator, {
		type ConnectionInfo
	} from '../common/alert/ConnectionIndicator.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import { createEventDispatcher, onDestroy, getContext, onMount } from 'svelte'
	import type { CaptureTriggerKind, Capture } from '$lib/gen'
	import CaptureIcon from './CaptureIcon.svelte'
	import Tooltip from '../Tooltip.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { Popover } from '$lib/components/meltComponents'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'

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
		stopCaptureListening()
	})

	/* function handleUpdateSchema(e: any) {
		dispatch('updateSchema', {
			payloadData: e.detail.payloadData,
			redirect: e.detail.redirect
		})
	} */

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

	let selectedCapture: any | undefined = undefined
	function handleSelectCapture(e: any) {
		if (e.detail && e.detail.id !== lastCapture?.id) {
			selectedCapture = e.detail
		} else if (!e.detail && lastCapture) {
			selectedCapture = lastCapture.payload
		}
	}

	// New code for capture fetching and management
	let lastCapture: Capture | undefined = undefined
	let newCaptureReceived = false
	let isLoadingBigPayload = false
	let capturePollingInterval: ReturnType<typeof setInterval> | undefined = undefined
	let lastCaptureId: number | undefined = undefined

	// Function to fetch the last capture when component mounts
	async function fetchLastCapture() {
		try {
			if (!captureInfo.path) return

			const captures = await CaptureService.listCaptures({
				workspace: $workspaceStore!,
				runnableKind: captureInfo.isFlow ? 'flow' : 'script',
				path: captureInfo.path,
				triggerKind: captureType,
				page: 1,
				perPage: 1
			})

			if (captures.length > 0) {
				lastCapture = captures[0]
				lastCaptureId = lastCapture.id

				// If it's a big payload, don't load it automatically
				if (lastCapture.payload !== 'WINDMILL_TOO_BIG') {
					selectedCapture = lastCapture.payload
				}
			}
		} catch (error) {
			console.error('Failed to fetch last capture:', error)
		}
	}

	// Function to listen for new captures (polls every 100ms)
	function listenForCaptures() {
		if (capturePollingInterval) return

		capturePollingInterval = setInterval(async () => {
			if (!captureInfo.active) return

			try {
				const captures = await CaptureService.listCaptures({
					workspace: $workspaceStore!,
					runnableKind: captureInfo.isFlow ? 'flow' : 'script',
					path: captureInfo.path,
					triggerKind: captureType,
					page: 1,
					perPage: 1
				})

				if (captures.length > 0 && lastCaptureId !== captures[0].id) {
					lastCapture = captures[0]
					lastCaptureId = lastCapture.id

					// Trigger animation for new capture
					showNewCaptureAnimation()

					// If it's not a large payload, automatically select it and display it
					if (lastCapture.payload !== 'WINDMILL_TOO_BIG') {
						selectedCapture = lastCapture.payload
					} else {
						// For big payloads, we can show the placeholder but keep existing selectedCapture if it exists
						if (!selectedCapture) {
							// This will trigger the UI to show the "Load large payload" button
							selectedCapture = undefined
						}
					}
				}
			} catch (error) {
				console.error('Error polling for new captures:', error)
			}
		}, 100)
	}

	// Stop listening for captures
	function stopCaptureListening() {
		if (capturePollingInterval) {
			clearInterval(capturePollingInterval)
			capturePollingInterval = undefined
		}
	}

	// Show animation when new capture arrives
	function showNewCaptureAnimation() {
		newCaptureReceived = true
		setTimeout(() => {
			newCaptureReceived = false
		}, 2000) // Animation duration
	}

	// Load big payload when requested
	async function loadBigPayload() {
		if (!lastCapture) return

		try {
			isLoadingBigPayload = true
			const fullCapture = await CaptureService.getCapture({
				workspace: $workspaceStore!,
				id: lastCapture.id
			})

			selectedCapture = fullCapture.payload
			isLoadingBigPayload = false
		} catch (error) {
			sendUserToast('Failed to load large payload', true)
			isLoadingBigPayload = false
		}
	}

	// Start or stop capture listening based on active state
	$: if (captureInfo.active) {
		listenForCaptures()
	} else {
		stopCaptureListening()
	}

	// Fetch last capture when component mounts
	onMount(() => {
		fetchLastCapture()
	})
</script>

<Splitpanes>
	<Pane class="flex flex-col gap-1 mb-4 pr-2 py-2" size={50}>
		<div class="flex flex-col gap-1 mb-4">
			<div class="flex justify-center w-full">
				<div class="relative h-fit">
					<AnimatedButton
						animate={captureInfo.active}
						wrapperClasses={captureInfo.active ? 'm-[-2px]' : ''}
						baseRadius="7px"
					>
						<Button
							size="xs"
							on:click={() => dispatch('captureToggle', {})}
							{disabled}
							color={captureInfo.active ? 'light' : 'dark'}
							btnClasses={captureInfo.active ? 'text-blue-500' : ''}
						>
							<div class="flex flex-row items-center gap-1 w-[160px] justify-center">
								{#if captureInfo.active}
									<CircleStop size={14} />
								{:else}
									<CaptureIcon variant="redDot" size={14} />
								{/if}
								{captureInfo.active ? 'Stop capturing' : 'Start capturing'}
							</div>
						</Button>
					</AnimatedButton>

					<div class="absolute top-1/2 -translate-y-1/2 -right-5">
						{#if captureInfo.active}
							<ConnectionIndicator connectionInfo={captureInfo.connectionInfo} />
						{:else}
							<!-- TODO: add tooltip  directly on hover the button-->
							<Tooltip>
								Start capturing to test your runnables with real data. Once active, all incoming
								payloads will be captured and displayed below, allowing you to test your runnables
								effectively.
							</Tooltip>
						{/if}
					</div>
				</div>
			</div>

			{#if disabled}
				<div class="text-sm font-normal text-red-600 dark:text-red-400" transition:slide>
					Enter a valid configuration to start capturing.
				</div>
			{/if}

			{#if $$slots.default}
				<div class:opacity-50={disabled || !captureInfo.active} class="grow min-h-0">
					<slot />
				</div>
			{/if}
		</div>
	</Pane>

	<Pane class="py-2 pl-2 flex flex-col">
		<div class="flex flex-row gap-1">
			<Popover
				placement="left"
				contentClasses="w-48"
				floatingConfig={{
					placement: 'left-start',
					offset: { mainAxis: 8, crossAxis: -4.5 },
					gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
				}}
			>
				<svelte:fragment slot="trigger">
					<Button size="xs2" color="light" iconOnly startIcon={{ icon: History }} nonCaptureEvent
					></Button>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<CaptureTable
						{captureType}
						bind:this={captureTable}
						isFlow={captureInfo.isFlow}
						path={captureInfo.path}
						on:select={handleSelectCapture}
						fullHeight={false}
						headless
						addButton={false}
					/>
				</svelte:fragment>
			</Popover>
		</div>

		<div class="flex flex-col gap-2 h-full">
			{#if lastCapture && lastCapture.payload === 'WINDMILL_TOO_BIG' && !selectedCapture}
				<div class="bg-surface flex flex-col items-center gap-2">
					<div class="text-amber-500 flex items-center gap-2">
						<AlertCircle size={20} />
						<span>Large payload detected</span>
					</div>
					<Button color="dark" loading={isLoadingBigPayload} on:click={loadBigPayload}>
						Load large payload
					</Button>
				</div>
			{:else if selectedCapture}
				<div
					class="bg-surface p-3 rounded-md text-sm overflow-auto max-h-[500px] grow shadow-sm"
					class:animate-highlight={newCaptureReceived}
				>
					<pre class="whitespace-pre-wrap break-words"
						>{JSON.stringify(selectedCapture, null, 2)}</pre
					>
				</div>
			{:else}
				<div class="text-center text-tertiary p-4 bg-surface rounded-md">No capture selected</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>

<style>
	@keyframes highlight {
		0% {
			color: rgba(59, 130, 246, 1);
			background-color: rgba(59, 130, 246, 0.1);
		}
		100% {
			color: inherit;
		}
	}

	.animate-highlight {
		animation: highlight 2s ease-out forwards;
	}
</style>
