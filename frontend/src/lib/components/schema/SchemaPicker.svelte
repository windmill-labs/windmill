<script lang="ts">
	import CustomPopover from '$lib/components/CustomPopover.svelte'
	import { copyToClipboard } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { twMerge } from 'tailwind-merge'

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
</script>

<button
	class={twMerge(
		'w-full rounded-sm px-2',
		hovering ? 'bg-surface-hover' : '',
		selected ? 'bg-surface-selected' : ''
	)}
	on:click={() => {
		dispatch('select')
	}}
	on:mouseenter={() => {
		hovering = true
	}}
	on:mouseleave={() => {
		hovering = false
	}}
>
	<div class="flex flex-row gap-2">
		<slot name="start" />

		<div
			class="text-2xs font-normal text-left p-2 rounded-md overflow-auto grow-0 text-ellipsis whitespace-nowrap scrollbar-none"
			title={formatDate(date)}
		>
			{formatDate(date)}
		</div>

		<CustomPopover class="grow min-w-12 ">
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div
				class={twMerge(
					'text-xs border font-normal text-left p-1 rounded-md overflow-auto grow whitespace-nowrap scrollbar-none',
					hovering && 'border-surface'
				)}
				on:click={() => {
					if (selected) {
						copyToClipboard(JSON.stringify(payloadData))
					}
				}}
				on:mouseenter={() => {
					hovering = true
				}}
				on:mouseleave={() => {
					hovering = false
				}}
			>
				{JSON.stringify(payloadData)}
			</div>
			<svelte:fragment slot="overlay">
				<ObjectViewer json={payloadData} />
			</svelte:fragment>
		</CustomPopover>

		{#if hovering}
			<slot name="extra" />
		{/if}
	</div>
</button>

<style>
	.scrollbar-none {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}

	.scrollbar-none::-webkit-scrollbar {
		display: none; /* Chrome, Safari and Opera */
	}
</style>
