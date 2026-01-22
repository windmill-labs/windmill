<script lang="ts">
	import type { AssetUsageAccessType, AssetUsageKind } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { formatAssetAccessType, getAssetUsagePageUri } from './lib'

	let usagesDrawerData:
		| {
				usages?: {
					path: string
					kind: AssetUsageKind
					access_type?: AssetUsageAccessType
				}[]
				runtime_usage_count?: number
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
		<h2 class="text-xs font-semibold text-emphasis mb-2">Static usages</h2>
		{#if !usagesDrawerData?.usages?.length}
			<p class="text-sm text-secondary mb-4">No static usages found for this asset.</p>
		{/if}
		<ul
			class={twMerge(
				'flex flex-col border rounded-md divide-y',
				!usagesDrawerData?.usages?.length ? 'hidden' : ''
			)}
		>
			{#each usagesDrawerData?.usages ?? [] as u}
				<li>
					<a
						href={getAssetUsagePageUri(u)}
						aria-label={`${u.kind}/${u.path}`}
						class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-3 hover:bg-surface-hover cursor-pointer"
					>
						<RowIcon kind={u.kind} />
						<div class="flex flex-col justify-center flex-1">
							<span class="font-semibold text-emphasis">{u.path}</span>
							<span class="text-2xs text-secondary">{u.kind}</span>
						</div>
						<div
							class={twMerge(
								'text-xs font-normal border text-primary w-10 p-1 text-center rounded-md',
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
