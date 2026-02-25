<script lang="ts">
	import { Drawer, DrawerContent } from '../common'
	import { VolumeService, type Volume } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { displayDate, displaySize, sendUserToast } from '$lib/utils'
	import { ExternalLink, Loader2, Trash2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import type S3FilePicker from '../S3FilePicker.svelte'

	let { s3FilePicker }: { s3FilePicker?: S3FilePicker } = $props()

	let open = $state(false)
	let volumeName = $state('')
	let loading = $state(false)
	let volume: Volume | undefined = $state(undefined)

	export function openDrawer(name: string) {
		volumeName = name
		open = true
		loadVolume()
	}

	async function loadVolume() {
		loading = true
		try {
			const volumes = await VolumeService.listVolumes({ workspace: $workspaceStore! })
			volume = volumes.find((v) => v.name === volumeName)
		} finally {
			loading = false
		}
	}

	async function deleteVolume() {
		if (!confirm(`Delete volume '${volumeName}'? This cannot be undone.`)) return
		await VolumeService.deleteVolume({ workspace: $workspaceStore!, name: volumeName })
		sendUserToast(`Volume '${volumeName}' deleted`)
		open = false
	}

	function exploreFiles() {
		open = false
		s3FilePicker?.open({ s3: `volumes/${volumeName}/` })
	}
</script>

<Drawer {open} size="600px" on:close={() => (open = false)}>
	<DrawerContent title="Volume: {volumeName}" on:close={() => (open = false)}>
		{#if loading}
			<div class="flex items-center justify-center py-8">
				<Loader2 size={24} class="animate-spin text-secondary" />
			</div>
		{:else if volume}
			<div class="flex flex-col gap-4">
				<div class="flex flex-col gap-2 border rounded-md p-4">
					<div class="flex justify-between text-sm">
						<span class="text-secondary">Files</span>
						<span>{volume.file_count}</span>
					</div>
					<div class="flex justify-between text-sm">
						<span class="text-secondary">Size</span>
						<span>{displaySize(volume.size_bytes) ?? '0 B'}</span>
					</div>
					<div class="flex justify-between text-sm">
						<span class="text-secondary">Created at</span>
						<span>{displayDate(volume.created_at)}</span>
					</div>
					<div class="flex justify-between text-sm">
						<span class="text-secondary">Created by</span>
						<span>{volume.created_by}</span>
					</div>
					{#if volume.last_used_at}
						<div class="flex justify-between text-sm">
							<span class="text-secondary">Last used</span>
							<span>{displayDate(volume.last_used_at)}</span>
						</div>
					{/if}
				</div>

				<div class="flex gap-2">
					{#if s3FilePicker}
						<Button
							variant="border"
							startIcon={{ icon: ExternalLink }}
							on:click={exploreFiles}
						>
							Explore files
						</Button>
					{/if}
					{#if $userStore?.is_admin}
						<Button
							variant="border"
							btnClasses="text-red-500 hover:text-red-600"
							startIcon={{ icon: Trash2 }}
							on:click={deleteVolume}
						>
							Delete
						</Button>
					{/if}
				</div>
			</div>
		{:else}
			<div class="text-sm text-secondary py-8 text-center">
				Volume '{volumeName}' not found.
			</div>
		{/if}
	</DrawerContent>
</Drawer>
