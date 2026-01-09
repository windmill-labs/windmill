<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let paginated = false
	export let currentPage = 1
	export let showNext = true

	const dispatch = createEventDispatcher()
</script>

<!-- A custom table 
- the first slot should be a <tr>, containing th elements
- the second slot should be a <tbody>, containing th elements
-->
<div class="flex flex-col {$$props.class} min-w-full">
	<div class="inline-block min-w-full py-2 align-middle">
		<table class="table-custom min-w-full table-auto divide-y">
			<thead>
				<slot name="header-row" />
			</thead>
			<slot name="body" />
		</table>
	</div>
	{#if paginated}
		<div class="sticky flex flex-row-reverse text-primary mb-6">
			<button
				class="ml-2 drop-shadow-md {showNext ? 'visible' : 'invisible'}"
				on:click={() => dispatch('next')}
			>
				Next
			</button>
			<button
				class="mx-2 drop-shadow-md {currentPage === 1 ? 'hidden' : ''}"
				on:click={() => dispatch('previous')}
			>
				Previous
			</button>
		</div>
	{/if}
</div>
