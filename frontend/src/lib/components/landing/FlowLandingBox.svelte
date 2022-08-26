<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Flow } from '$lib/gen'

	export let flow: Flow
</script>

<div
	class="border p-4 rounded-sm shadow-sm space-y-2 hover:border-blue-600 flex flex-col justify-between cursor-pointer"
	on:click={() => goto(`/flows/get/${flow.path}`)}
>
	<div class="font-bold">{flow.summary}</div>

	<div class="inline-flex justify-between w-full h-16">
		<div class="text-xs">{flow.path}</div>
	</div>

	<div
		class="inline-flex space-x-2 space-x-reverse flex-row-reverse w-full bottom-4 right-4 flex-wrap"
	>
		<button
			on:click|stopPropagation={() => goto(`/flows/edit/${flow.path}?step=2`)}
			type="button"
			class="inline-flex items-center text-gray-700 bg-transparent border border-gray-700 hover:bg-gray-800 hover:text-white focus:ring-4 focus:outline-none focus:ring-gray-300 font-medium rounded-lg text-xs px-3 py-1.5 text-center"
			data-dismiss-target="#alert-additional-content-5"
			aria-label="Close"
		>
			<svg
				class="w-4 h-4 mr-2"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
				xmlns="http://www.w3.org/2000/svg"
				><path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
				/>
			</svg>
			Edit
		</button>

		<button
			on:click|stopPropagation={() => goto(`/flows/run/${flow.path}`)}
			type="button"
			class="inline-flex items-center text-gray-700 bg-transparent border border-gray-700 hover:bg-gray-800 hover:text-white focus:ring-4 focus:outline-none focus:ring-gray-300 font-medium rounded-lg text-xs px-3 py-1.5 text-center"
			data-dismiss-target="#alert-additional-content-5"
			aria-label="Close"
		>
			<svg
				class="w-4 h-4 mr-2"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
				xmlns="http://www.w3.org/2000/svg"
				><path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
				/><path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
				/>
			</svg>
			Run
		</button>
	</div>
</div>
