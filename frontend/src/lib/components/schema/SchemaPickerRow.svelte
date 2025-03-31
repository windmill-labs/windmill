<script lang="ts">
	import { copyToClipboard, formatDate, formatDateShort } from '$lib/utils'
	import ObjectViewerWrapper from '$lib/components/propertyPicker/ObjectViewerWrapper.svelte'
	import { twMerge } from 'tailwind-merge'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { CopyIcon, Eye, Loader2 } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { isObjectTooBig } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import type { FloatingConfig } from '@melt-ui/svelte/internal/actions'

	export let payloadData: Record<string, any> | string
	export let date: string | undefined
	export let hovering = false
	export let placement: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end' = 'bottom-start'
	export let viewerOpen = false
	export let limitPayloadSize = false
	export let forceLoad = false

	let popover: Popover | undefined = undefined
	let popoverOpen = false
	let objectViewerLoaded = false
	let popoverFullyOpened = false
	let popoverOpenTimeout: ReturnType<typeof setTimeout> | null = null

	const dispatch = createEventDispatcher()
	const payloadTooBigForPreview = payloadData != 'WINDMILL_TOO_BIG' && isObjectTooBig(payloadData)
	const isTooBig = payloadData === 'WINDMILL_TOO_BIG' || payloadTooBigForPreview

	const floatingConfig: FloatingConfig = {
		placement,
		strategy: 'fixed',
		offset: { mainAxis: 4 },
		gutter: 0,
		fitViewport: true
	}

	function handlePopoverChange(event: CustomEvent<boolean>) {
		const isOpen = event.detail
		popoverOpen = isOpen

		if (!isOpen) {
			objectViewerLoaded = false
			popoverFullyOpened = false
			forceLoad = false
		} else {
			// Set a small delay to ensure the popover is fully rendered before
			// marking it as fully opened
			setTimeout(() => {
				popoverFullyOpened = true
			}, 50)
		}

		dispatch('openChange', isOpen)
	}

	function handleHoveringChange(isHovering: boolean) {
		if (isHovering && viewerOpen && popover && !popoverOpen) {
			popoverOpenTimeout = setTimeout(() => {
				popover?.open()
				popoverOpenTimeout = null
			}, 100)
		} else if (!isHovering && popoverOpenTimeout) {
			clearTimeout(popoverOpenTimeout)
			popoverOpenTimeout = null
		}
	}

	$: handleHoveringChange(hovering)
</script>

<Cell>
	<slot name="start" />
</Cell>

<Cell
	wrap
	class="text-2xs font-normal text-left py-2 overflow-auto text-ellipsis scrollbar-none"
	title={formatDate(date)}
>
	{formatDateShort(date)}
</Cell>

<Cell class="items-center flex flex-row gap-2">
	<div class="flex items-center justify-center border grow min-w-0 rounded-md">
		<div
			class={twMerge(
				'grow min-w-0 text-xs p-1 font-normal text-tertiary text-left  whitespace-nowrap overflow-hidden text-ellipsis',
				hovering && 'border-surface'
			)}
		>
			{JSON.stringify(payloadData)}
		</div>

		<Popover
			bind:this={popover}
			class="w-fit"
			contentClasses="overflow-auto"
			usePointerDownOutside
			closeOnOtherPopoverOpen
			on:click={(e) => {
				e.stopPropagation()
			}}
			{floatingConfig}
			on:openChange={handlePopoverChange}
		>
			<svelte:fragment slot="trigger">
				<Button
					variant="contained"
					size="xs2"
					color="light"
					btnClasses="bg-transparent hover:bg-surface"
					nonCaptureEvent
				>
					<Eye size={16} />
				</Button>
			</svelte:fragment>

			<svelte:fragment slot="content">
				<div class="relative p-2 max-w-[400px]">
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
							class="w-full h-full"
							role="button"
							tabindex="0"
							aria-label="Copy JSON payload to clipboard"
							on:click={() => {
								copyToClipboard(JSON.stringify(payloadData))
							}}
							on:keydown
						>
							{#if !objectViewerLoaded && isTooBig}
								<div class="flex justify-center items-center p-4">
									<Loader2 size={20} class="animate-spin text-primary" />
									<span class="ml-2 text-xs text-tertiary">Loading data...</span>
								</div>
							{/if}

							{#if popoverFullyOpened || !isTooBig}
								<div
									class={(!objectViewerLoaded && isTooBig ? 'invisible h-0 overflow-hidden' : '') +
										' pr-6'}
								>
									<ObjectViewerWrapper
										json={payloadData}
										allowCopy
										pureViewer
										on:mounted={() => (objectViewerLoaded = true)}
									/>
								</div>
							{/if}

							<div class="absolute top-2 right-2">
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
	</div>

	<slot name="extra" {isTooBig} />
</Cell>

<style>
	.scrollbar-none {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}

	.scrollbar-none::-webkit-scrollbar {
		display: none; /* Chrome, Safari and Opera */
	}
</style>
