<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Script } from '$lib/gen'

	import { truncateHash } from '$lib/utils'

	export let script: Script
</script>

<div class="border p-4 rounded-md shadow-sm space-y-2 hover:shadow-md">
	<div class="font-bold">{script.summary}</div>

	<div class="inline-flex justify-between w-full">
		<div class="text-xs">{script.path}</div>

		<span class="bg-gray-100 text-gray-800 text-xs font-semibold mr-2 px-2.5 py-0.5 rounded ">
			{truncateHash(script.hash)}
		</span>
	</div>
	<div class="inline-flex space-x-1 w-full">
		<span class="bg-blue-100 text-blue-800 text-xs font-semibold  px-2.5 py-0.5 rounded ">
			{String(script.language).toUpperCase()}
		</span>
		{#if script.is_trigger}
			<span class="bg-red-100 text-red-800 text-xs font-semibold px-2.5 py-0.5 rounded">
				TRIGGER
			</span>
		{/if}
	</div>
	<div class="inline-flex space-x-2 space-x-reverse flex-row-reverse w-full">
		<button
			on:click={() => goto(`/scripts/edit/${script.hash}?step=2`)}
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
			on:click={() => goto(`/scripts/run/${script.hash}`)}
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
