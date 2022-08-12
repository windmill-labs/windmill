<script lang="ts">
	import type { FlowModule } from '$lib/gen'

	import {
		faFlag,
		faFlagCheckered,
		faPen,
		faPlus,
		faRotate,
		faSliders
	} from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import typescript from 'svelte-highlight/languages/typescript'
	import ModuleStep from '../ModuleStep.svelte'
	import FlowInput from './FlowInput.svelte'
	import { addModule, flowStore } from './flowStore'

	// export let flow: Flow
	export let args: Record<string, any> = {}
	export let open: number
	export let modules: FlowModule[]
	export let root: boolean = true

	function handleNewModule(index: number) {
		addModule(index)
	}

	const color = root ? 'blue' : 'orange'
</script>

<div class="w-full">
	<ol class="relative ml-4 border-l border-gray-200 dark:border-gray-700 space-y-12 border-dashed">
		{#if root}
			<li class="ml-8">
				<span class="relative">
					<span
						class={`flex absolute -left-12 justify-center items-center w-8 h-8 bg-${color}-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
					>
						<Icon
							class={`text-${color}-600 dark:text-${color}-400  text-font-bold text-center`}
							data={faSliders}
						/>
					</span>
				</span>
				<slot name="settings" />
			</li>
			<li class="ml-8 ">
				<span class="relative">
					<span
						class={`flex absolute top-4 -left-12 justify-center items-center w-8 h-8 bg-${color}-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
					>
						<Icon
							class={`text-${color}-600 dark:text-${color}-400 font-bold text-center`}
							data={faPen}
						/>
					</span>
				</span>
				<FlowInput />
			</li>
		{/if}
		{#each modules as module, index}
			{#if module.value.type === 'forloopflow'}
				<li class="ml-4 relative">
					<div
						class="p-4 pl-12 mb-4 text-sm font-bold text-orange-700 bg-orange-100 rounded-lg dark:bg-orange-200 dark:text-orange-800"
						role="alert"
					>
						For loop (Step {index + 1})
					</div>
					<span
						class="flex absolute top-3 -left-8 justify-center items-center w-8 h-8 bg-orange-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-orange-900"
					>
						<Icon
							class="text-orange-600 dark:text-orange-400 font-bold text-center"
							data={faRotate}
						/>
					</span>
					<div class="ml-12">
						<div
							class="p-4 mb-4 text-sm text-blue-700 bg-blue-100 rounded-lg dark:bg-blue-200 dark:text-blue-800"
							role="alert"
						>
							Inside a loop, the flow input has an "iter" property. It contains the value and the
							index of the iteration.
						</div>
						{#if module.stop_after_if_expr}
							<Highlight language={typescript} code={module.stop_after_if_expr} />
						{/if}
						{#if module.skip_if_stopped}
							<span
								class="bg-green-100 text-green-800 text-sm font-medium mr-2 px-2.5 py-0.5 rounded dark:bg-green-200 dark:text-green-900"
							>
								Skip if stopped
							</span>
						{/if}
					</div>
					<svelte:self bind:args bind:open modules={module.value.value.modules} root={false} />
					<span
						class="flex absolute bottom-3 -left-8 justify-center items-center w-8 h-8 bg-orange-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-orange-900"
					>
						<Icon
							class="text-orange-600 dark:text-orange-400 font-bold text-center"
							data={faFlagCheckered}
						/>
					</span>

					<div class="ml-12">
						<div
							class="p-4 mt-4 text-sm text-blue-700 bg-blue-100 rounded-lg dark:bg-blue-200 dark:text-blue-800"
							role="alert"
						>
							The results of each iteration are collecting in an array and is the result of the
							step.
						</div>
					</div>

					<div
						class="p-4 pl-12 mt-4 text-sm font-bold text-orange-700 bg-orange-100 rounded-lg dark:bg-orange-200 dark:text-orange-800"
						role="alert"
					>
						End of For Loop
					</div>
				</li>
			{:else}
				<li class="ml-8">
					<span class="relative">
						<button
							on:click={() => handleNewModule(index)}
							class="flex absolute -top-10 -left-12 justify-center items-center bg-white border-2 border-gray-400 w-8 h-8 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900"
						>
							<Icon class="text-gray-400" data={faPlus} />
						</button>
						<span
							class={`flex absolute top-4 -left-12 justify-center items-center w-8 h-8 bg-${color}-200  rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
						>
							<span class={`text-${color}-600 dark:text-${color}-400 font-bold text-center`}
								>{index + 1}</span
							>
						</span>
						<ModuleStep bind:open bind:mod={module} bind:args i={index} />
						{#if $flowStore?.value.modules.length - 1 === index}
							<span
								class={`flex absolute bottom-0 -left-12 justify-center items-center w-8 h-8 bg-${color}-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
							>
								<Icon
									class={`text-${color}-600 dark:text-${color}-400 font-bold text-center`}
									data={faFlag}
								/>
							</span>
						{/if}
					</span>
				</li>
			{/if}
		{/each}
	</ol>
</div>
