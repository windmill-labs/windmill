<script lang="ts">
	import type { AssetUsageAccessType, AssetUsageKind } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { formatAssetAccessType, getAssetUsagePageUri } from './lib'

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
						<div
							class={twMerge(
								'text-xs font-normal border text-tertiary w-10 p-1 text-center rounded-md',
								!u.access_type ? 'hover:bg-surface active:opacity-80' : ''
							)}
						>
							{formatAssetAccessType(u.access_type)}
						</div>
					</a>
				</li>
			{/each}
		</ul>
	</DrawerContent>
</Drawer>
