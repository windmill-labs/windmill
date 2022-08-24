<script>
	import { goto } from '$app/navigation'
</script>

<div class="p-4 border border-gray-300 rounded-md bg-gray-50 dark:border-gray-600 dark:bg-gray-700">
	<div class="flex items-center">
		<h3 class="inline-flex items-center text-lg font-medium text-gray-700 ">
			<svg
				class="w-6 h-6 mr-2"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
				xmlns="http://www.w3.org/2000/svg"
				><path d="M12 14l9-5-9-5-9 5 9 5z" /><path
					d="M12 14l6.16-3.422a12.083 12.083 0 01.665 6.479A11.952 11.952 0 0012 20.055a11.952 11.952 0 00-6.824-2.998 12.078 12.078 0 01.665-6.479L12 14z"
				/><path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M12 14l9-5-9-5-9 5 9 5zm0 0l6.16-3.422a12.083 12.083 0 01.665 6.479A11.952 11.952 0 0012 20.055a11.952 11.952 0 00-6.824-2.998 12.078 12.078 0 01.665-6.479L12 14zm-4 6v-7.5l4-2.222"
				/>
			</svg>
			Getting started
		</h3>
	</div>
	<div class="mt-2 mb-4 text-sm text-gray-700 dark:text-gray-300">
		Connect to apps like Slack, Google Drive or Airtable using OAuth.
	</div>
	<div class="flex">
		<button
			on:click={() => goto('resources')}
			type="button"
			class="inline-flex items-center text-white bg-gray-700 hover:bg-gray-800 focus:ring-4 focus:outline-none focus:ring-gray-300 font-medium rounded-lg text-xs px-3 py-1.5 mr-2 text-center  dark:bg-gray-600 dark:hover:bg-gray-500 dark:focus:ring-gray-600"
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
					d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
				/>
			</svg>
			Connect an API
		</button>
	</div>
</div>
