<script lang="ts">
	export let close: () => void
</script>

<div class="relative">
	<div class="p-3 bg-white rounded-lg border border-gray-200 shadow-md">
		<div id="alert-4" class="flex p-4 bg-yellow-100 rounded-lg dark:bg-yellow-200" role="alert">
			<svg
				class="flex-shrink-0 w-5 h-5 text-yellow-700 dark:text-yellow-800"
				fill="currentColor"
				viewBox="0 0 20 20"
				xmlns="http://www.w3.org/2000/svg"
				><path
					fill-rule="evenodd"
					d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
					clip-rule="evenodd"
				/></svg
			>
			<div class="ml-3 text-sm font-medium text-yellow-700 dark:text-yellow-800">
				The property picker is not available unless a preview is run on the previous step.
			</div>
			<button
				type="button"
				class="ml-auto -mx-1.5 -my-1.5 bg-yellow-100 text-yellow-500 rounded-lg focus:ring-2 focus:ring-yellow-400 p-1.5 hover:bg-yellow-200 inline-flex h-8 w-8 dark:bg-yellow-200 dark:text-yellow-600 dark:hover:bg-yellow-300"
				data-dismiss-target="#alert-4"
				aria-label="Close"
				on:click={close}
			>
				<span class="sr-only">Close</span>
				<svg
					class="w-5 h-5"
					fill="currentColor"
					viewBox="0 0 20 20"
					xmlns="http://www.w3.org/2000/svg"
					><path
						fill-rule="evenodd"
						d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
						clip-rule="evenodd"
					/></svg
				>
			</button>
		</div>
	</div>
</div>
