<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { json } from 'svelte-highlight/languages'
	import Highlight from 'svelte-highlight'
	import { twMerge } from 'tailwind-merge'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { CopyIcon } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let payloadData: Record<string, any> | string
	export let date: string | undefined
	export let hovering = false

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
	<Popover
		class="w-full overflow-auto flex items-center justify-center"
		contentClasses="max-w-[50vh] overflow-auto max-h-[50vh] min-w-60 min-h-28 "
		placement="bottom-start"
		usePointerDownOutside
		closeOnOtherPopoverOpen
	>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<svelte:fragment slot="trigger">
			<div
				class={twMerge(
					'text-xs border w-full font-normal text-tertiary text-left p-1 rounded-md whitespace-nowrap overflow-hidden text-ellipsis',
					hovering && 'border-surface'
				)}
			>
				{JSON.stringify(payloadData)}
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content">
			{#if payloadData === 'WINDMILL_TOO_BIG'}
				<div class="text-center text-tertiary text-xs">
					Payload too big to preview but can still be loaded
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
					<Highlight
						class={'h-full w-full'}
						language={json}
						code={JSON.stringify(payloadData ?? null, null, 4) ?? 'null'}
					/>

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
		</svelte:fragment>
	</Popover>

	<slot name="extra" />
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
