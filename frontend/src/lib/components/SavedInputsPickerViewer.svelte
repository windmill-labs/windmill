<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ObjectViewerWrapper from '$lib/components/propertyPicker/ObjectViewerWrapper.svelte'
	import { copyToClipboard, isObjectTooBig } from '$lib/utils'
	import { Eye, CopyIcon, Loader2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let payloadData: any
	export let limitPayloadSize: boolean = false
	export let hover: boolean = false
	export let viewerOpen: boolean = false
	export let maxWidth: number | undefined = undefined
	export let editOptions: boolean = true

	const dispatch = createEventDispatcher()
	const payloadTooBigForPreview = payloadData != 'WINDMILL_TOO_BIG' && isObjectTooBig(payloadData)
	const buttonWidth = 34
	const floatingConfig = {
		placement: 'bottom-end',
		strategy: 'fixed',
		offset: { mainAxis: 8, crossAxis: -buttonWidth },
		gutter: 0,
		middleware: [
			{
				name: 'flip',
				options: {
					fallbackPlacements: ['bottom-top']
				}
			}
		]
	}
	const xOffset = editOptions ? 218 : 168 // width of the optional buttons on the right

	let popover: Popover | undefined
	let popoverOpen = false
	let hoverTimeout: ReturnType<typeof setTimeout> | undefined
	let objectViewerLoaded = false
	let popoverFullyOpened = false

	function handlePopoverChange(event: CustomEvent<boolean>) {
		const isOpen = event.detail
		popoverOpen = isOpen

		if (!isOpen) {
			objectViewerLoaded = false
			popoverFullyOpened = false
		} else {
			// Set a small delay to ensure the popover is fully rendered before
			// marking it as fully opened
			setTimeout(() => {
				popoverFullyOpened = true
			}, 50)
		}

		dispatch('openChange', isOpen)
	}

	function handleHoverChange(isHovering: boolean) {
		if (isHovering) {
			// Start debounce timer when hovering starts
			hoverTimeout = setTimeout(() => {
				if (viewerOpen && popover && !popoverOpen) {
					popover.open()
				}
			}, 100)
		} else {
			// Clear timeout if hover ends before debounce period
			if (hoverTimeout) {
				clearTimeout(hoverTimeout)
				hoverTimeout = undefined
			}
		}
	}

	$: handleHoverChange(hover)
	$: ajustedWidth = maxWidth ? Math.abs(maxWidth - xOffset) : undefined
</script>

<Popover
	bind:this={popover}
	{floatingConfig}
	on:openChange={handlePopoverChange}
	on:click={(e) => {
		e.stopPropagation()
	}}
	closeOnOtherPopoverOpen
	closeOnClickOutside={false}
	usePointerDownOutside
>
	<svelte:fragment slot="trigger" let:isOpen>
		<Button
			variant="contained"
			color="light"
			btnClasses={hover || isOpen ? 'block -my-2 hover:bg-surface bg-transparent' : 'hidden'}
			wrapperClasses="p-0"
			size="xs2"
			iconOnly
			nonCaptureEvent
			startIcon={{ icon: Eye }}
		/>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div
			class="p-2 overflow-auto"
			style="width: {ajustedWidth ? ajustedWidth + 'px' : '50vh'}; max-height: 50vh"
		>
			{#if payloadData === 'WINDMILL_TOO_BIG'}
				<div class="text-center text-tertiary text-xs">
					{#if limitPayloadSize}
						Payload too big to be used
					{:else}
						Payload too big to preview but can still be loaded
					{/if}
				</div>
			{:else}
				<div
					class="relative p-2"
					role="button"
					tabindex="0"
					aria-label="Copy JSON payload to clipboard"
					on:click={() => {
						copyToClipboard(JSON.stringify(payloadData))
					}}
					on:keydown
				>
					{#if !objectViewerLoaded && payloadTooBigForPreview}
						<div class="flex justify-center items-center py-4">
							<Loader2 size={20} class="animate-spin text-primary" />
							<span class="ml-2 text-xs text-tertiary">Loading data...</span>
						</div>
					{/if}

					{#if popoverFullyOpened || !payloadTooBigForPreview}
						<div
							class={!objectViewerLoaded && payloadTooBigForPreview
								? 'invisible h-0 overflow-hidden'
								: ''}
						>
							<ObjectViewerWrapper
								json={payloadData}
								allowCopy
								pureViewer
								on:mounted={() => (objectViewerLoaded = true)}
							/>
						</div>
					{/if}

					<div class="absolute top-2 right-2 w-full h-full">
						<Button
							variant="contained"
							size="xs2"
							class="absolute top-0 right-0"
							iconOnly
							startIcon={{ icon: CopyIcon }}
							nonCaptureEvent
						/>
					</div>
				</div>
			{/if}
		</div>
	</svelte:fragment>
</Popover>
