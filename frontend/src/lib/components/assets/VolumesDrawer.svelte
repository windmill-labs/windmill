<script lang="ts">
	import { Alert, Drawer, DrawerContent } from '../common'
	import { VolumeService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore, userStore } from '$lib/stores'
	import { displayDate, displaySize, emptyString, sendUserToast } from '$lib/utils'
	import { File, HardDriveIcon, Loader2, Plus, Shield, Trash2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import SharedBadge from '../SharedBadge.svelte'
	import ShareModal from '../ShareModal.svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { resource } from 'runed'

	let { onExplore }: { onExplore?: (volumeName: string) => void } = $props()

	let open = $state(false)
	let refreshKey = $state(0)
	let shareModal: ShareModal | undefined = $state()
	let newVolumeName = $state('')

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

	async function createVolume(name: string, close: () => void) {
		try {
			await VolumeService.createVolume({
				workspace: $workspaceStore!,
				requestBody: { name }
			})
			sendUserToast(`Volume '${name}' created`)
			newVolumeName = ''
			close()
			refreshKey++
		} catch (e) {
			sendUserToast(`Failed to create volume: ${e}`, true)
		}
	}

	async function deleteVolume(name: string) {
		if (!confirm(`Delete volume '${name}'? This cannot be undone.`)) return
		await VolumeService.deleteVolume({ workspace: $workspaceStore!, name })
		sendUserToast(`Volume '${name}' deleted`)
		refreshKey++
	}

	function canReadVolume(
		createdBy: string,
		extraPerms?: { [key: string]: unknown }
	): boolean {
		if ($userStore?.is_admin) return true
		const username = $userStore?.username
		if (username === createdBy || `u/${username}` === createdBy) return true
		const perms = extraPerms ?? {}
		const keys = Object.keys(perms)
		if (keys.length === 0) return true // public
		if (`u/${username}` in perms) return true
		const pgroups = $userStore?.pgroups ?? []
		return pgroups.some((g) => g in perms)
	}

	function canWriteVolume(
		createdBy: string,
		extraPerms?: { [key: string]: unknown }
	): boolean {
		if ($userStore?.is_admin) return true
		const username = $userStore?.username
		if (username === createdBy || `u/${username}` === createdBy) return true
		if (extraPerms?.[`u/${username}`] === true) return true
		const pgroups = $userStore?.pgroups ?? []
		return pgroups.some((g) => extraPerms?.[g] === true)
	}
</script>

<Drawer {open} size="700px" on:close={() => (open = false)}>
	<DrawerContent title="Volumes" on:close={() => (open = false)}>
		{#snippet actions()}
			<Popover
				floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
				containerClasses="border rounded-lg shadow-lg bg-surface"
			>
				{#snippet trigger()}
					<Button unifiedSize="sm" variant="accent" startIcon={{ icon: Plus }} nonCaptureEvent
						>New volume</Button
					>
				{/snippet}
				{#snippet content({ close })}
					<div class="flex flex-col gap-2 p-4">
						<TextInput
							size="md"
							inputProps={{
								placeholder: 'Volume name',
								onkeyup: (e) => {
									if (e.key === 'Enter' && newVolumeName.trim()) {
										createVolume(newVolumeName.trim(), close)
									}
								}
							}}
							bind:value={newVolumeName}
						/>
						<Button
							unifiedSize="sm"
							variant="accent"
							startIcon={{ icon: Plus }}
							disabled={!newVolumeName.trim()}
							on:click={() => createVolume(newVolumeName.trim(), close)}
						>
							Create
						</Button>
					</div>
				{/snippet}
			</Popover>
		{/snippet}
		{#if emptyString($enterpriseLicense)}
			<div class="mb-4">
				<Alert type="info" title="Community Edition limits">
					Volumes are limited to 20 per workspace and 50 MB per file. Upgrade to Enterprise
					Edition to remove these limits.
				</Alert>
			</div>
		{/if}
		{#if volumes.loading}
			<div class="flex items-center justify-center py-8">
				<Loader2 size={24} class="animate-spin text-secondary" />
			</div>
		{:else if !volumes.current?.length}
			<div class="text-sm text-secondary py-8 text-center">
				No volumes yet. Create one above or they are auto-created when a job declares a volume annotation.
			</div>
		{:else}
			<div class="flex flex-col divide-y border rounded-md">
				{#each volumes.current as vol (vol.name)}
					{@const readable = canReadVolume(vol.created_by, vol.extra_perms)}
					{@const writable = canWriteVolume(vol.created_by, vol.extra_perms)}
					<div class="flex items-center gap-3 px-4 py-3 hover:bg-surface-hover">
						<HardDriveIcon size={16} class="text-secondary shrink-0" />
						<div class="flex flex-col flex-1 min-w-0">
							<div class="flex items-center gap-1.5">
								<span class="text-sm font-medium truncate">{vol.name}</span>
								<SharedBadge
									extraPerms={vol.extra_perms as Record<string, boolean>}
									canWrite={writable}
								/>
							</div>
							<span class="text-2xs text-secondary">
								{vol.file_count} {vol.file_count === 1 ? 'file' : 'files'}
								&middot; {displaySize(vol.size_bytes) ?? '0 B'}
								&middot; owner: {vol.created_by.replace(/^u\//, '')}
							</span>
						</div>
						{#if vol.last_used_at}
							<span class="text-2xs text-secondary shrink-0">
								Used {displayDate(vol.last_used_at)}
							</span>
						{/if}
						{#if writable}
							<Button
								variant="subtle"
								iconOnly
								unifiedSize="sm"
								endIcon={{ icon: Shield }}
								on:click={() =>
									shareModal?.openDrawer(vol.name, 'volume', true)}
							/>
						{/if}
						{#if onExplore && readable}
							<Button
								variant="default"
								unifiedSize="sm"
								endIcon={{ icon: File }}
								on:click={() => onExplore(vol.name)}
							>
								Explore
							</Button>
						{/if}
						{#if writable}
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

<ShareModal bind:this={shareModal} on:change={() => refreshKey++} />
