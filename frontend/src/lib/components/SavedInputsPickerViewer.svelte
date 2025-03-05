<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { copyToClipboard, isObjectTooBig } from '$lib/utils'
	import { Eye, CopyIcon } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let payloadData: any
	export let limitPayloadSize: boolean = false
	export let forceLoad: boolean = false
	export let hover: boolean = false
	export let viewerOpen: boolean = false
	export let maxWidth: number | undefined = undefined
	export let editOptions: boolean = true

	const dispatch = createEventDispatcher()
	const payloadTooBigForPreview = payloadData != 'WINDMILL_TOO_BIG' || isObjectTooBig(payloadData)
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

	$: if (hover && viewerOpen && popover && !popoverOpen) {
		popover.open()
	}
	$: ajustedWidth = maxWidth ? Math.abs(maxWidth - xOffset) : undefined
</script>

<Popover
	bind:this={popover}
	{floatingConfig}
	on:openChange={({ detail }) => {
		popoverOpen = detail
		dispatch('openChange', detail)
		forceLoad = false
	}}
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
					class="relative p-2"
					role="button"
					tabindex="0"
					aria-label="Copy JSON payload to clipboard"
					on:click={() => {
						copyToClipboard(JSON.stringify(payloadData))
					}}
					on:keydown
				>
					<ObjectViewer json={payloadData} />

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
