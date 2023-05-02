<script lang="ts">
	import { getContext } from 'svelte'

	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../editor/appUtils'

	export let id: string
	export let componentInput: AppInput | undefined

	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let result: string[] | undefined = undefined

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let currentIndex = 1
</script>

<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
	<nav aria-label="Progress">
		<ol class="flex items-center flex-row">
			{#each result ?? [] as step, index}
				{#if index < currentIndex}
					<li class="flex flex-row w-32 items-center">
						<div class="flex flex-row gap-2 items-center truncate w-full">
							<a
								href="#"
								class="relative flex h-4 w-4 items-center justify-center rounded-full bg-indigo-600 hover:bg-indigo-900"
							>
								<svg
									class="h-5 w-5 text-white"
									viewBox="0 0 20 20"
									fill="currentColor"
									aria-hidden="true"
								>
									<path
										fill-rule="evenodd"
										d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z"
										clip-rule="evenodd"
									/>
								</svg>
							</a>
							<span>{step}</span>
						</div>
						<div class="h-0.5 bg-indigo-600 w-16" />
					</li>
				{:else if index === currentIndex}
					<li class="relative pr-8 sm:pr-20">
						<!-- Current Step -->
						<div class="absolute inset-0 flex items-center" aria-hidden="true">
							<div class="h-0.5 w-full bg-gray-200" />
						</div>
						<a
							href="#"
							class="relative flex h-6 w-6 items-center justify-center rounded-full border-2 border-indigo-600 bg-white"
							aria-current="step"
						>
							<span class="h-2.5 w-2.5 rounded-full bg-indigo-600" aria-hidden="true" />
							<span class="sr-only">{step}</span>
						</a>
					</li>
				{:else if currentIndex < index && index === (result ?? []).length - 1}
					<li class="relative">
						<!-- Upcoming Step -->
						<div class="absolute inset-0 flex items-center" aria-hidden="true">
							<div class="h-0.5 w-full bg-gray-200" />
						</div>
						<a
							href="#"
							class="group relative flex h-6 w-6 items-center justify-center rounded-full border-2 border-gray-300 bg-white hover:border-gray-400"
						>
							<span
								class="h-2.5 w-2.5 rounded-full bg-transparent group-hover:bg-gray-300"
								aria-hidden="true"
							/>
							<span class="sr-only">{step}</span>
						</a>
					</li>
				{:else}
					<li class="relative pr-8 sm:pr-20">
						<!-- Upcoming Step -->
						<div class="absolute inset-0 flex items-center" aria-hidden="true">
							<div class="h-0.5 w-full bg-gray-200" />
						</div>
						<a
							href="#"
							class="group relative flex h-6 w-6 items-center justify-center rounded-full border-2 border-gray-300 bg-white hover:border-gray-400"
						>
							<span
								class="h-2.5 w-2.5 rounded-full bg-transparent group-hover:bg-gray-300"
								aria-hidden="true"
							/>
							<span class="sr-only">Step 4</span>
						</a>
					</li>
				{/if}
			{/each}
		</ol>
	</nav>
</RunnableWrapper>
