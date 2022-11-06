<script lang="ts">
	import type { CompletedJob, QueuedJob } from '$lib/gen'

	import JobStatus from '../JobStatus.svelte'
	export let job: QueuedJob | CompletedJob
</script>

<div class="overflow-x-auto relative">
	<table class="text-sm text-left text-gray-500 dark:text-gray-400">
		<tbody>
			<tr>
				<th scope="row" class="py-2 pr-6 font-bold text-gray-900 whitespace-nowrap dark:text-white">
					Status
				</th>
				<td class="py-2 "> <JobStatus {job} /></td>
			</tr>
			{#if job}
				<tr>
					<th scope="row" class="py-2 pr-6  font-bold text-gray-900 whitespace-nowrap ">
						Job Id
					</th>
					<td class="py-2">
						<a rel="noreferrer" target="_blank" href="/run/{job?.id}">
							{job?.id}
						</a>
					</td>
				</tr>
			{/if}
		</tbody>
	</table>
</div>
