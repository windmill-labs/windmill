<script lang="ts">
	import type { CompletedJob } from '$lib/gen'

	import ChevronButton from './ChevronButton.svelte'
	import DisplayResult from './DisplayResult.svelte'

	export let job: CompletedJob | undefined
</script>

{#if job}
	<div class="flex flex-col ml-10">
		<div>
			<ChevronButton text="result" viewOptions={true}>
				<div class="text-xs">
					<DisplayResult result={job.result} />
				</div>
			</ChevronButton>
		</div>
		<div>
			<ChevronButton text="logs" viewOptions={true}>
				<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-lg">
					<pre class="w-full">{job.logs}</pre>
				</div>
			</ChevronButton>
		</div>
	</div>
{/if}
