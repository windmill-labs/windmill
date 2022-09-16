<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'

	export let flowModule: FlowModule
</script>

<div class="font-bold text-md mb-2">Suspend</div>
<div class="space-y-4 ">
	<div class="p-4 mb-4 border border-blue-300 rounded-lg bg-blue-50 ">
		<div class="flex items-center">
			<svg
				aria-hidden="true"
				class="w-5 h-5 mr-2 text-blue-900"
				fill="currentColor"
				viewBox="0 0 20 20"
				xmlns="http://www.w3.org/2000/svg"
				><path
					fill-rule="evenodd"
					d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
					clip-rule="evenodd"
				/></svg
			>
			<span class="sr-only">Info</span>
			<h3 class="text-lg font-medium text-blue-900">Suspend</h3>
		</div>
		<div class="mt-2 mb-4 text-sm text-blue-900">
			The optional suspend value is a non-negative integer that determines the number of events
			(resume messages) needed to progress to the next step in the flow.
		</div>
		<div class="flex">
			<a
				href="https://docs.windmill.dev/docs/openflow/"
				class="text-white bg-blue-900 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-200 font-medium rounded-lg text-xs px-3 py-1.5 mr-2 text-center inline-flex items-center "
			>
				<svg
					aria-hidden="true"
					class="-ml-0.5 mr-2 h-4 w-4"
					fill="currentColor"
					viewBox="0 0 20 20"
					xmlns="http://www.w3.org/2000/svg"
				>
					<path d="M10 12a2 2 0 100-4 2 2 0 000 4z" />
					<path
						fill-rule="evenodd"
						d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z"
						clip-rule="evenodd"
					/>
				</svg>
				Learn more
			</a>
		</div>
	</div>

	<input bind:value={flowModule.suspend} type="number" min="0" placeholder="0" />

	{#if flowModule.stop_after_if}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => {
				flowModule.stop_after_if = undefined
			}}
		>
			Disable stop expression
		</button>
		<div class="flex flex-col">
			<div class="font-bold text-md mb-2">Suspend</div>

			<Toggle
				bind:checked={flowModule.stop_after_if.skip_if_stopped}
				options={{
					right: 'Skip if stopped'
				}}
			/>
			<div class="font-bold text-md mb-2">Suspend</div>
			<input bind:value={flowModule.stop_after_if.expr} />
		</div>
	{:else}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => {
				flowModule.stop_after_if = {
					skip_if_stopped: false,
					expr: 'false'
				}
			}}
		>
			Enable stop expression
		</button>
	{/if}
</div>
