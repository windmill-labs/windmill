<script lang="ts">
	import ArgInfo from './ArgInfo.svelte'
	import { Skeleton } from './common'
	import TableCustom from './TableCustom.svelte'

	export let args: any
</script>

<TableCustom class="py-2">
	<tr slot="header-row">
		<th>Argument</th>
		<th>Value</th>
	</tr>
	<tbody slot="body">
		{#if args && Object.keys(args).length > 0}
			{#each Object.entries(args) as [arg, value]}
				<tr>
					<td class="font-semibold">{arg}</td>
					<td><ArgInfo {value} /></td>
				</tr>
			{/each}
		{:else if args}
			<tr><div class="text-gray-600 pt-2 pl-1 text-sm">No arguments</div></tr>
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
