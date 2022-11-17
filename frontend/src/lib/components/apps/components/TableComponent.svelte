<script lang="ts">
	import type {} from '$lib/common'
	import { classNames } from '$lib/utils'

	export let title: string
	export let description: string | undefined = undefined

	export let headers: string[]
	export let data: Array<Record<string, any>>

	export const staticOutputs: string[] = []

	let query: string = ''
	let page: number = 1
</script>

<div class="p-8 w-full">
	<div class="sm:flex sm:items-center">
		<div class="sm:flex-auto">
			<h1 class="text-xl font-semibold text-gray-900">{title}</h1>
			{#if description}
				<p class="mt-2 text-sm text-gray-700">
					{description}
				</p>
			{/if}
		</div>
	</div>
	<div class="my-4 flex flex-col">
		<div class="-my-2 -mx-4 overflow-x-auto sm:-mx-6 lg:-mx-8">
			<div class="inline-block min-w-full py-2 align-middle md:px-6 lg:px-8">
				<div class="overflow-hidden shadow ring-1 ring-black ring-opacity-5 md:rounded-lg">
					<table class="min-w-full divide-y divide-gray-300">
						<thead class="bg-gray-50">
							<tr>
								{#each headers as header}
									<th
										scope="col"
										class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>
										{header}
									</th>
								{/each}
								<th scope="col" class="relative py-3.5 pl-3 pr-4 sm:pr-6">
									<span class="sr-only">Edit</span>
								</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-gray-200 bg-white">
							{#each data as x}
								<tr>
									{#each headers as header}
										<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
											{x[header]}
										</td>
									{/each}
									<td
										class="relative whitespace-nowrap py-4 pl-3 pr-4 text-right text-sm font-medium sm:pr-6"
									>
										<a href="#" class="text-indigo-600 hover:text-indigo-900">
											Edit
											<span class="sr-only">, Lindsay Walton </span>
										</a>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		</div>
	</div>

	<nav>
		<ul class="inline-flex -space-x-px">
			<li>
				<button
					on:click={() => (page -= 1)}
					class="text-sm py-2 px-4 text-gray-500 bg-white rounded-l-lg border border-gray-300 hover:bg-gray-100 hover:text-gray-700"
				>
					Previous
				</button>
			</li>

			{#each Array(5) as x, i}
				<li>
					<button
						on:click={() => (page = i)}
						class={classNames(
							'text-sm py-2 px-4 text-gray-500 bg-white border border-gray-300 hover:bg-gray-100 hover:text-gray-700',
							page === i ? 'bg-blue-100 font-bold' : 'bg-white'
						)}
					>
						{i}
					</button>
				</li>
			{/each}

			<li>
				<button
					on:click={() => (page += 1)}
					class="text-sm py-2 px-4 text-gray-500 bg-white rounded-r-lg border border-gray-300 hover:bg-gray-100 hover:text-gray-700 dark:bg-gray-800 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
				>
					Next
				</button>
			</li>
		</ul>
	</nav>
</div>
