<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	// A table suitable if you can pass data as a list of row objects
	export let headers: string[] | undefined
	export let data: any[] | undefined // Object containing the data
	export let keys: string[]
	export let defaultText: string = 'No data to display'
	export let paginated = false
	export let twTextSize: string = 'text-sm md:text-base'

	const dispatch = createEventDispatcher()
</script>

<div class="mt-2 flex flex-col {$$props.class}">
	<div class="inline-block min-w-full align-middle">
		<table class="min-w-full divide-y divide-gray-300 table-auto">
			<thead>
				<tr class={twTextSize}>
					{#if headers}
						{#each headers as header, i}
							<th
								class="py-3.5 text-left text-sm font-semibold text-gray-900 capitalize {i == 0
									? 'sm:pl-6 md:pl-0 pl-4 pr-3'
									: 'px-3 '}">{header}</th
							>
						{/each}
					{/if}
				</tr>
			</thead>
			<tbody class="divide-y divide-gray-200">
				{#if data && keys && data.length > 0}
					{#each data as row}
						<tr>
							{#each keys as key, i}
								<td
									class="py-2 text-sm text-gray-700 break-words {i == 0
										? 'pl-4 pr-3 sm:pl-6 md:pl-0 font-semibold'
										: 'px-3'} {twTextSize}"
								>
									{row[key] ?? ''}</td
								>
							{/each}
						</tr>
					{/each}
				{:else if data}
					<tr>{defaultText}</tr>
				{:else}
					<tr>Loading...</tr>
				{/if}
			</tbody>
		</table>
	</div>
</div>
{#if paginated}
	<div class="flex flex-row-reverse text-gray-500">
		<button
			on:click={() => {
				dispatch('next')
			}}>Next</button
		>
		<button
			class="mx-2"
			on:click={() => {
				dispatch('next')
			}}>Previous</button
		>
	</div>
{/if}
