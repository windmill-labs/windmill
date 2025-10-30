<script lang="ts">
	import CustomPopover from '$lib/components/CustomPopover.svelte'
	import { copyToClipboard } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		payloadData: Record<string, any>;
		date: string | undefined;
		selected?: boolean;
		start?: import('svelte').Snippet;
		extra?: import('svelte').Snippet;
	}

	let {
		payloadData,
		date,
		selected = false,
		start,
		extra
	}: Props = $props();

	const dispatch = createEventDispatcher()

	let hovering = $state(false)
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
	onclick={() => {
		dispatch('select')
	}}
	onmouseenter={() => {
		hovering = true
	}}
	onmouseleave={() => {
		hovering = false
	}}
>
	<div class="flex flex-row gap-2">
		{@render start?.()}

		<div
			class="text-2xs font-normal text-left p-2 rounded-md overflow-auto grow-0 text-ellipsis whitespace-nowrap scrollbar-none"
			title={formatDate(date)}
		>
			{formatDate(date)}
		</div>

		<CustomPopover class="grow min-w-12 ">
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class={twMerge(
					'text-xs border font-normal text-left p-1 rounded-md overflow-auto grow whitespace-nowrap scrollbar-none',
					hovering && 'border-surface'
				)}
				onclick={() => {
					if (selected) {
						copyToClipboard(JSON.stringify(payloadData))
					}
				}}
				onmouseenter={() => {
					hovering = true
				}}
				onmouseleave={() => {
					hovering = false
				}}
			>
				{JSON.stringify(payloadData)}
			</div>
			{#snippet overlay()}
					
					<ObjectViewer json={payloadData} />
				
					{/snippet}
		</CustomPopover>

		{#if hovering}
			{@render extra?.()}
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
