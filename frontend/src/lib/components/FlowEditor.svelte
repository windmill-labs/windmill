<script lang="ts">
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import FlowPreview from './FlowPreview.svelte'
	import CopyFirstStepSchema from './flows/CopyFirstStepSchema.svelte'
	import { addModule, flowStore } from './flows/flowStore'
	import ModuleStep from './ModuleStep.svelte'
	import SchemaEditor from './SchemaEditor.svelte'

	let args: Record<string, any> = {}
	$: numberOfSteps = $flowStore?.value.modules.length - 1
</script>

<div class="flow-root bg-gray-50 rounded-xl border  border-gray-200">
	<ul class="relative -mt-10">
		<div class="relative">
			<li class="flex flex-row flex-shrink max-w-full mx-auto mt-20">
				<div class="bg-white border border-gray xl-rounded shadow-lg w-full mx-4 xl:mx-20">
					<div
						class="flex items-center justify-between flex-wra px-4 py-5 border-b border-gray-200 sm:px-6"
					>
						<h3 class="text-lg leading-6 font-medium text-gray-900">Flow Input</h3>
						<CopyFirstStepSchema />
					</div>
					<div class="p-4">
						<SchemaEditor bind:schema={$flowStore.schema} />
						<div class="my-4" />
						<FlowPreview bind:flow={$flowStore} i={numberOfSteps} bind:args />
					</div>
				</div>
			</li>
			{#each $flowStore?.value.modules as mod, i}
				<ModuleStep bind:mod bind:args {i} />
			{/each}
			<li class="relative m-20 ">
				<div class="relative flex justify-center">
					<button
						class="default-button h-10 w-10 shadow-blue-600/40  border-blue-600 shadow"
						on:click={() => addModule()}
					>
						<Icon class="text-white mb-1" data={faPlus} />
						Add step
					</button>
				</div>
			</li>
		</div>
	</ul>
</div>
<div class="py-10 bg-white" />
