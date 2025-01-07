<script lang="ts">
	import CustomPopover from '$lib/components/CustomPopover.svelte'
	import { copyToClipboard } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { twMerge } from 'tailwind-merge'
	import Row from '$lib/components/table/Row.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	export let payloadData: Record<string, any>
	export let date: string | undefined
	export let selected = false

	const dispatch = createEventDispatcher()

	let hovering = false
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

<Row
	class={twMerge(
		hovering ? 'bg-surface-hover' : '',
		selected ? 'bg-surface-selected' : '',
		'cursor-pointer'
	)}
	on:click={() => {
		dispatch('select')
	}}
	bind:hovering
>
	<Cell>
		<slot name="start" />
	</Cell>

	<Cell
		wrap
		class="text-2xs font-normal text-left p-2 rounded-md overflow-auto text-ellipsis scrollbar-none"
		title={formatDate(date)}
	>
		{formatDateShort(date)}
	</Cell>

	<Cell class="items-center flex">
		<CustomPopover class="w-full overflow-auto flex items-center justify-center">
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div
				class={twMerge(
					'text-xs border w-full font-normal text-left p-1 rounded-md whitespace-nowrap overflow-hidden text-ellipsis',
					hovering && 'border-surface'
				)}
				on:click={() => {
					if (selected) {
						copyToClipboard(JSON.stringify(payloadData))
					}
				}}
			>
				{JSON.stringify(payloadData)}
			</div>
			<svelte:fragment slot="overlay">
				<ObjectViewer json={payloadData} />
			</svelte:fragment>
		</CustomPopover>

		<slot name="extra" />
	</Cell>
</Row>

<style>
	.scrollbar-none {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}

	.scrollbar-none::-webkit-scrollbar {
		display: none; /* Chrome, Safari and Opera */
	}
</style>
