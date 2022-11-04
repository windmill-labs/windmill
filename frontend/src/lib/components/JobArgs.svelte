<script lang="ts">
	import type { Job } from '$lib/gen'
	import ArgInfo from './ArgInfo.svelte'
	import { Skeleton } from './common'
	import TableCustom from './TableCustom.svelte'

	export let job: Job | undefined
</script>

<TableCustom class="px-10 py-4">
	<tr slot="header-row"
		><th>Argument</th>
		<th>Value</th></tr
	>
	<tbody slot="body">
		{#if job && job.args && Object.keys(job.args).length > 0}
			{#each Object.entries(job.args) as [arg, value]}
				<tr>
					<td class="font-semibold">{arg}</td>
					<td><ArgInfo {value} /></td>
				</tr>
			{/each}
		{:else if job}
			<tr>No arguments</tr>
		{:else}
			<tr>
				<td>
					<Skeleton layout={[[3], 0.5, [3]]} />
				</td>
				<td>
					<Skeleton layout={[[3], 0.5, [3]]} />
				</td>
			</tr>
		{/if}
	</tbody>
</TableCustom>
