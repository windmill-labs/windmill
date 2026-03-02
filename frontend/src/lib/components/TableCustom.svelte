<script lang="ts">
	interface Props {
		paginated?: boolean
		currentPage?: number
		showNext?: boolean
		class?: string
		header_row?: import('svelte').Snippet
		body?: import('svelte').Snippet
		onNext?: () => void
		onPrevious?: () => void
	}

	let {
		paginated = false,
		currentPage = 1,
		showNext = true,
		class: className = '',
		header_row,
		body,
		onNext,
		onPrevious
	}: Props = $props()
</script>

<!-- A custom table
- the first slot should be a <tr>, containing th elements
- the second slot should be a <tbody>, containing th elements
-->
<div class="flex flex-col {className} min-w-full">
	<div class="inline-block min-w-full py-2 align-middle">
		<table class="table-custom min-w-full table-auto divide-y">
			<thead>
				{@render header_row?.()}
			</thead>
			{@render body?.()}
		</table>
	</div>
	{#if paginated}
		<div class="sticky flex flex-row-reverse text-primary mb-6">
			<button
				class="ml-2 drop-shadow-md {showNext ? 'visible' : 'invisible'}"
				onclick={() => onNext?.()}
			>
				Next
			</button>
			<button
				class="mx-2 drop-shadow-md {currentPage === 1 ? 'hidden' : ''}"
				onclick={() => onPrevious?.()}
			>
				Previous
			</button>
		</div>
	{/if}
</div>
