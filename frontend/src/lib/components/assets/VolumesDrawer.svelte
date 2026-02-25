<script lang="ts">
	import { Drawer, DrawerContent } from '../common'
	import { VolumeService } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { displayDate, displaySize, sendUserToast } from '$lib/utils'
	import { File, HardDriveIcon, Loader2, Trash2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { resource } from 'runed'

	let { onExplore }: { onExplore?: (volumeName: string) => void } = $props()

	let open = $state(false)
	let refreshKey = $state(0)

	let volumes = resource(
		() => (open ? { ws: $workspaceStore, key: refreshKey } : undefined),
		(params) => {
			if (!params?.ws) return Promise.resolve([])
			return VolumeService.listVolumes({ workspace: params.ws })
		}
	)

	export function openDrawer() {
		open = true
	}

	async function deleteVolume(name: string) {
		if (!confirm(`Delete volume '${name}'? This cannot be undone.`)) return
		await VolumeService.deleteVolume({ workspace: $workspaceStore!, name })
		sendUserToast(`Volume '${name}' deleted`)
		refreshKey++
	}
</script>

<Drawer {open} size="700px" on:close={() => (open = false)}>
	<DrawerContent title="Volumes" on:close={() => (open = false)}>
		{#if volumes.loading}
			<div class="flex items-center justify-center py-8">
				<Loader2 size={24} class="animate-spin text-secondary" />
			</div>
		{:else if !volumes.current?.length}
			<div class="text-sm text-secondary py-8 text-center">
				No volumes yet. Volumes are auto-created when a job declares a volume annotation.
			</div>
		{:else}
			<div class="flex flex-col divide-y border rounded-md">
				{#each volumes.current as vol (vol.name)}
					<div class="flex items-center gap-3 px-4 py-3 hover:bg-surface-hover">
						<HardDriveIcon size={16} class="text-secondary shrink-0" />
						<div class="flex flex-col flex-1 min-w-0">
							<span class="text-sm font-medium truncate">{vol.name}</span>
							<span class="text-2xs text-secondary">
								{vol.file_count} {vol.file_count === 1 ? 'file' : 'files'}
								&middot; {displaySize(vol.size_bytes) ?? '0 B'}
								&middot; created by {vol.created_by}
							</span>
						</div>
						{#if vol.last_used_at}
							<span class="text-2xs text-secondary shrink-0">
								Used {displayDate(vol.last_used_at)}
							</span>
						{/if}
						{#if onExplore}
							<Button
								variant="default"
								unifiedSize="sm"
								endIcon={{ icon: File }}
								on:click={() => {
									open = false
									onExplore(vol.name)
								}}
							>
								Explore
							</Button>
						{/if}
						{#if $userStore?.is_admin}
							<Button
								variant="subtle"
								iconOnly
								unifiedSize="sm"
								btnClasses="text-red-500 hover:text-red-600"
								endIcon={{ icon: Trash2 }}
								on:click={() => deleteVolume(vol.name)}
							/>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</DrawerContent>
</Drawer>
