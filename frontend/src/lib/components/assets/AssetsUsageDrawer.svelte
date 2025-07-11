<script lang="ts">
	import type { AssetUsageAccessType, AssetUsageKind } from '$lib/gen'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import {
		assetDisplaysAsInputInFlowGraph,
		assetDisplaysAsOutputInFlowGraph
	} from '../graph/renderers/nodes/AssetNode.svelte'
	import { getAssetUsagePageUri } from './lib'

	let usagesDrawerData:
		| {
				usages: {
					path: string
					kind: AssetUsageKind
					access_type?: AssetUsageAccessType
				}[]
		  }
		| undefined = $state()

	export function open(data: typeof usagesDrawerData) {
		usagesDrawerData = data
	}
</script>

<Drawer
	open={usagesDrawerData !== undefined}
	size="900px"
	on:close={() => (usagesDrawerData = undefined)}
>
	<DrawerContent title="Asset usage" on:close={() => (usagesDrawerData = undefined)}>
		<ul class="flex flex-col border rounded-md divide-y">
			{#each usagesDrawerData?.usages ?? [] as u}
				<li>
					<a
						href={getAssetUsagePageUri(u)}
						aria-label={`${u.kind}/${u.path}`}
						class="text-sm text-primary flex items-center py-3 px-4 gap-3 hover:bg-surface-hover cursor-pointer"
					>
						<RowIcon kind={u.kind} />
						<div class="flex flex-col justify-center flex-1">
							<span class="font-semibold">{u.path}</span>
							<span class="text-xs text-tertiary">{u.kind}</span>
						</div>
						<div class="flex gap-2">
							{#if assetDisplaysAsInputInFlowGraph(u)}
								<div class="text-xs border text-tertiary max-w-fit p-1 rounded-md">Read</div>
							{/if}
							{#if assetDisplaysAsOutputInFlowGraph(u)}
								<div class="text-xs border text-tertiary max-w-fit p-1 rounded-md">Write</div>
							{/if}
						</div>
					</a>
				</li>
			{/each}
		</ul>
	</DrawerContent>
</Drawer>
