<script module lang="ts">
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
	import { fade } from 'svelte/transition'
	import AnimatedButton from '../common/button/AnimatedButton.svelte'
	import PulseButton from '../common/button/PulseButton.svelte'
	import Button from '../common/button/Button.svelte'
	import { CircleStop, History, Play, Loader2 } from 'lucide-svelte'
	import ConnectionIndicator, {
		type ConnectionInfo
	} from '../common/alert/ConnectionIndicator.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import { createEventDispatcher, onDestroy, getContext, onMount, untrack } from 'svelte'
	import type { CaptureTriggerKind, Capture } from '$lib/gen'
	import CaptureIcon from './CaptureIcon.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { Popover } from '$lib/components/meltComponents'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { isObject, sendUserToast } from '$lib/utils'
	import { triggerIconMap } from './utils'
	import { formatDateShort } from '$lib/utils'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import DisplayResultControlBar from '$lib/components/DisplayResultControlBar.svelte'
	import { base } from '$lib/base'
	import Description from '$lib/components/Description.svelte'
	import { twMerge } from 'tailwind-merge'
	import { FlaskConical } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'

	interface Props {
		disabled?: boolean | undefined
		captureType: CaptureTriggerKind
		captureInfo: CaptureInfo
		hasPreprocessor?: boolean
		isFlow?: boolean
		captureLoading?: boolean
		description?: import('svelte').Snippet
		children?: import('svelte').Snippet
		displayAlert?: boolean
	}

	let {
		disabled = undefined,
		captureType,
		captureInfo,
		hasPreprocessor = false,
		isFlow = false,
		captureLoading = false,
		description,
		children,
		displayAlert = false
	}: Props = $props()

	const testKind: 'preprocessor' | 'main' = $derived(hasPreprocessor ? 'preprocessor' : 'main')

	const dispatch = createEventDispatcher<{
		captureToggle: { disableOnly?: boolean }
		updateSchema: { payloadData: Record<string, any>; redirect: boolean; args?: boolean }
		addPreprocessor: null
		testWithArgs: Record<string, any>
		applyArgs: { kind: 'main' | 'preprocessor'; args: Record<string, any> }
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
	$effect(() => {
		updateShowCaptureHint($showCaptureHint)
	})

	let selectedCapture: Capture | undefined = $state(undefined)
	function handleSelectCapture(e: any) {
		if (e.detail) {
			selectCapture(e.detail)
		} else if (lastCapture) {
			selectCapture(lastCapture)
		}
	}

	// New code for capture fetching and management
	let lastCapture: Capture | undefined = $state(undefined)
	let newCaptureReceived = $state(false)
	let isLoadingBigPayload = $state(false)
	let capturePollingInterval: ReturnType<typeof setInterval> | undefined = undefined
	let lastCaptureId: number | undefined = undefined
	let displayResult: DisplayResult | undefined = $state(undefined)
	let toolbarLocation: 'internal' | 'external' | undefined = $state(undefined)

	function selectCapture(capture: Capture) {
		selectedCapture = capture
		if (
			capture.main_args === 'WINDMILL_TOO_BIG' ||
			capture.preprocessor_args === 'WINDMILL_TOO_BIG'
		) {
			loadBigPayload(capture)
		}
	}

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

				selectCapture(lastCapture)
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

					selectCapture(lastCapture)
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
	async function loadBigPayload(capture: Capture | undefined) {
		if (!capture) return

		try {
			isLoadingBigPayload = true
			const fullCapture = await CaptureService.getCapture({
				workspace: $workspaceStore!,
				id: capture.id
			})

			capture.main_args = fullCapture.main_args
			capture.preprocessor_args = fullCapture.preprocessor_args
			isLoadingBigPayload = false
		} catch (error) {
			sendUserToast('Failed to load large payload', true)
			isLoadingBigPayload = false
		}
	}

	function getCapturePayload(capture: Capture) {
		let payloadData: any = {}
		const preprocessor_args = isObject(capture.preprocessor_args) ? capture.preprocessor_args : {}
		if ('wm_trigger' in preprocessor_args) {
			// v1
			payloadData =
				testKind === 'preprocessor'
					? {
							...(typeof capture.main_args === 'object' ? capture.main_args : {}),
							...preprocessor_args
						}
					: capture.main_args
		} else {
			// v2
			payloadData = testKind === 'preprocessor' ? capture.preprocessor_args : capture.main_args
		}
		return payloadData
	}

	// Start or stop capture listening based on active state
	$effect(() => {
		if (captureInfo.active) {
			untrack(() => listenForCaptures())
		} else {
			untrack(() => stopCaptureListening())
		}
	})

	// Fetch last capture when component mounts
	onMount(() => {
		fetchLastCapture()
	})
</script>

<Splitpanes>
	<Pane size={50} minSize={30}>
		<div
			class="flex flex-col gap-1 px-4 py-2 h-full w-full overflow-auto"
			style="scrollbar-gutter: stable"
		>
			<div class="text-sm text-secondary flex items-center gap-1">
				<FlaskConical size={16} />
				Test trigger
			</div>
			<div class="flex flex-col gap-1 mb-4 w-full">
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
								startIcon={captureInfo.active
									? { icon: CircleStop }
									: { icon: CaptureIcon, props: { variant: 'redDot' } }}
								loading={captureLoading}
							>
								{#if captureInfo.active}
									<p class="w-24" transition:fade={{ duration: 300 }}>Stop capturing</p>
								{:else}
									<p class="w-24" transition:fade={{ duration: 300 }}>Start capturing</p>
								{/if}
							</Button>
						</AnimatedButton>

						<div class="absolute top-1/2 -translate-y-1/2 -right-5">
							{#if captureInfo.active}
								<ConnectionIndicator connectionInfo={captureInfo.connectionInfo} />
							{/if}
						</div>
					</div>
				</div>

				<div class="mt-4 mb-2">
					{#if displayAlert}
						<Alert type="warning" title="Trigger deployed" size="xs" class="mb-4">
							Capturing on a deployed trigger can cause event loss on the deployed trigger. Treat
							carefully.
						</Alert>
					{/if}
					<Description>
						<div class="relative min-h-8">
							{#key (captureInfo.active, disabled)}
								<div
									class={twMerge(
										'absolute top-0 left-0 w-full text-center',
										disabled === true ? 'text-red-600 dark:text-red-400' : ''
									)}
									in:fade={{ duration: 100, delay: 50 }}
									out:fade={{ duration: 50 }}
								>
									{#if disabled === true}
										Enter a valid configuration to start capturing.
									{:else}
										{@render description?.()}
									{/if}
								</div>
							{/key}
						</div>
					</Description>
				</div>

				{#if children}
					<div class="grow min-h-0 flex flex-col gap-4">
						{@render children?.()}
					</div>
				{/if}
			</div>
		</div>
	</Pane>

	<Pane minSize={30} class="flex flex-col">
		<div class="flex flex-row gap-1 justify-between min-h-[33.5px] pl-1 pr-4">
			<div class="flex flex-row gap-1 items-center">
				{#if lastCapture}
					<Popover
						placement="left"
						contentClasses="w-48 min-h-48 max-h-64 overflow-auto"
						floatingConfig={{
							placement: 'left-start',
							offset: { mainAxis: 8, crossAxis: -4.5 },
							gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
						}}
						usePointerDownOutside
					>
						{#snippet trigger()}
							<Button
								size="xs2"
								color="light"
								iconOnly
								startIcon={{ icon: History }}
								nonCaptureEvent
								btnClasses="h-[27px]"
							></Button>
						{/snippet}
						{#snippet content()}
							<CaptureTable
								{captureType}
								isFlow={captureInfo.isFlow}
								path={captureInfo.path}
								on:selectCapture={handleSelectCapture}
								fullHeight
								headless
								addButton={false}
								noBorder
							/>
						{/snippet}
					</Popover>
				{/if}
				{#if selectedCapture}
					{@const SvelteComponent = triggerIconMap[captureType]}
					<div
						class={'min-w-16 text-secondary flex flex-row w-fit items-center gap-2 rounded-md bg-surface-secondary p-1 px-2 h-[27px]'}
					>
						<SvelteComponent size={12} />
						<span class="text-xs text-secondary truncate">
							Capture {formatDateShort(selectedCapture?.created_at)}
						</span>
					</div>
				{/if}

				{#if selectedCapture}
					{@const label = isFlow && testKind === 'main' ? 'Test flow with args' : 'Apply args'}
					{@const title =
						isFlow && testKind === 'main'
							? 'Test flow using captured data'
							: testKind === 'preprocessor'
								? 'Apply args to preprocessor'
								: 'Apply args to inputs'}
					<Button
						size="xs2"
						color="dark"
						btnClasses="h-[27px]"
						dropdownItems={[
							{
								label: 'Use as input schema',
								onClick: async () => {
									if (!lastCapture) return
									const payloadData = selectedCapture?.main_args
									dispatch('updateSchema', {
										payloadData: payloadData ?? {},
										redirect: true,
										args: true
									})
								},
								disabled: !selectedCapture,
								hidden: !isFlow || testKind !== 'main'
							}
						].filter((item) => !item.hidden)}
						on:click={async () => {
							if (!selectedCapture) return
							const payloadData = selectedCapture?.main_args ?? {}
							if (isFlow && testKind === 'main') {
								dispatch('testWithArgs', payloadData)
							} else {
								const trigger_extra = isObject(selectedCapture.preprocessor_args)
									? selectedCapture.preprocessor_args
									: {}

								dispatch('applyArgs', {
									kind: testKind,
									args: { ...structuredClone(payloadData), ...trigger_extra }
								})
							}
						}}
						disabled={testKind === 'preprocessor' && !hasPreprocessor}
						{title}
						startIcon={isFlow && testKind === 'main' ? { icon: Play } : {}}
					>
						{label}
					</Button>
				{/if}
			</div>

			{#if displayResult && toolbarLocation === 'external'}
				<DisplayResultControlBar
					{base}
					result={selectedCapture?.main_args}
					disableTooltips={false}
					on:open-drawer={() => {
						if (displayResult && typeof displayResult.openDrawer === 'function') {
							displayResult.openDrawer()
						}
					}}
				/>
			{/if}
		</div>
		<div class="grow min-h-0 rounded-md w-full pl-2 py-1 pb-2 overflow-auto">
			{#if isLoadingBigPayload}
				<Loader2 class="animate-spin" />
			{:else if selectedCapture?.main_args}
				<div class="bg-surface rounded-md text-sm" class:animate-highlight={newCaptureReceived}>
					<DisplayResult
						bind:this={displayResult}
						workspaceId={undefined}
						jobId={undefined}
						result={getCapturePayload(selectedCapture)}
						externalToolbarAvailable
						on:toolbar-location-changed={({ detail }) => {
							toolbarLocation = detail
						}}
					/>
				</div>
			{:else}
				<div class="text-center text-tertiary p-4 bg-surface rounded-md"
					>No captures to show yet.</div
				>
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
