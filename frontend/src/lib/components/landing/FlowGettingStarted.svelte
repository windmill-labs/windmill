<script>
	import { goto } from '$app/navigation'
</script>

<div class="p-4 border border-gray-300 rounded-md bg-gray-50 ">
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
	<div class="mt-2 mb-4 text-sm text-gray-700 ">
		Flows allow you to streamline complex processes and operations by chaining simple steps
		together. Each Flow is composed of one or more steps. Create a new flow or find inspiration on
		the Hub!
	</div>
	<div class="flex space-x-2">
		<button
			type="button"
			on:click={() => goto('flows/add')}
			class="inline-flex items-center text-white bg-gray-700 hover:bg-gray-800 focus:ring-4 focus:outline-none focus:ring-gray-300 font-medium rounded-lg text-xs px-3 py-1.5 text-center"
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
					d="M12 6v6m0 0v6m0-6h6m-6 0H6"
				/></svg
			>
			Create flow
		</button>
		<button
			on:click={() => goto('https://docs.windmill.dev/docs/getting_started/flows')}
			type="button"
			class="inline-flex items-center text-white bg-blue-600 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-xs px-3 py-1.5 text-center"
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
					d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
				/>
			</svg>
			Flow documentation
		</button>
		<button
			type="button"
			class="inline-flex items-center text-gray-700 bg-transparent border border-gray-700 hover:bg-gray-800 hover:text-white focus:ring-4 focus:outline-none focus:ring-gray-300 font-medium rounded-lg text-xs px-3 py-1.5 text-center "
			data-dismiss-target="#alert-additional-content-5"
			aria-label="Close"
			on:click={() => goto('https://hub.windmill.dev/')}
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
					d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
				/>
			</svg>
			Explore community flows on WindmillHub
		</button>
	</div>
</div>
