<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { twMerge } from 'tailwind-merge'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { CopyIcon, Eye } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Placement } from '@floating-ui/dom'
	import { isObjectTooBig } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'

	export let payloadData: Record<string, any> | string
	export let date: string | undefined
	export let hovering = false
	export let placement: 'bottom-start' | 'top-start' = 'bottom-start'
	export let viewerOpen = false
	export let limitPayloadSize = false
	export let forceLoad = false

	let clientWidth = 0
	let popover: Popover | undefined = undefined
	let popoverOpen = false

	const buttonWidth = 34
	const dispatch = createEventDispatcher()
	const payloadTooBigForPreview = payloadData != 'WINDMILL_TOO_BIG' && isObjectTooBig(payloadData)
	const isTooBig = payloadData === 'WINDMILL_TOO_BIG' || payloadTooBigForPreview

	function formatDate(dateString: string | undefined): string {
		if (!dateString) return ''
		const date = new Date(dateString)
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}).format(date)
	}

	function formatDateShort(dateString: string | undefined): string {
		if (!dateString) return ''
		const date = new Date(dateString)
		const now = new Date()

		// If date is today, only show time
		if (date.toDateString() === now.toDateString()) {
			return new Intl.DateTimeFormat('en-US', {
				hour: '2-digit',
				minute: '2-digit'
			}).format(date)
		}

		// If date is this year, show only month and day
		if (date.getFullYear() === now.getFullYear()) {
			return new Intl.DateTimeFormat('en-US', {
				month: 'short',
				day: 'numeric'
			}).format(date)
		}

		// If date is from another year, only show the date with year
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		}).format(date)
	}

	const fallbackPlacements: Placement[] =
		placement === 'top-start' ? ['bottom-start'] : ['top-start']
	const floatingConfig = {
		placement,
		strategy: 'fixed',
		offset: { mainAxis: 4, crossAxis: -buttonWidth },
		gutter: 0,
		middleware: [
			{
				name: 'flip',
				options: {
					fallbackPlacements
				}
			}
		]
	}

	$: if (hovering && viewerOpen && popover && !popoverOpen) {
		popover.open()
	}
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
	<div class="flex items-center justify-center border grow min-w-0 rounded-md" bind:clientWidth>
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
			usePointerDownOutside
			closeOnOtherPopoverOpen
			on:click={(e) => {
				e.stopPropagation()
			}}
			{floatingConfig}
			on:openChange={({ detail }) => {
				popoverOpen = detail
				forceLoad = false
				dispatch('openChange', detail)
			}}
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
				<div
					class="relative p-2 overflow-auto max-h-[40vh]"
					style={`width: ${clientWidth - buttonWidth}px; min-width: ${clientWidth - buttonWidth}px`}
				>
					{#if payloadData === 'WINDMILL_TOO_BIG'}
						<div class="text-center text-tertiary text-xs">
							{#if limitPayloadSize}
								Payload too big to be used
							{:else}
								Payload too big to preview but can still be loaded
							{/if}
						</div>
					{:else if payloadTooBigForPreview && !forceLoad}
						<div class="text-center text-tertiary text-xs">
							Payload too big for preview
							{#if limitPayloadSize}or for use here{/if}.

							<button class="text-disabled hover:underline" on:click={() => (forceLoad = true)}>
								Load preview anyway
							</button>
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
							<ObjectViewer json={payloadData} allowCopy pureViewer />

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
