<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		Badge,
		Button,
		Drawer,
		DrawerContent,
		Skeleton,
		Tab,
		TabContent,
		Tabs
	} from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { sendUserToast } from '$lib/toast'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { Plus, Trash, Upload, RefreshCw, Info } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { displayDate } from '$lib/utils'

	type Snapshot = {
		workspace_id: string
		name: string
		tag: string
		s3_key: string
		content_hash: string
		docker_image: string
		setup_script: string | null
		size_bytes: number | null
		status: string
		build_error: string | null
		build_job_id: string | null
		created_by: string
		created_at: string
		updated_at: string
		extra_perms: Record<string, any>
	}

	type Volume = {
		workspace_id: string
		name: string
		s3_key: string
		size_bytes: number | null
		created_by: string
		created_at: string
		updated_at: string
		extra_perms: Record<string, any>
	}

	let tab: string = $state('snapshots')
	let snapshots: Snapshot[] | undefined = $state(undefined)
	let volumes: Volume[] | undefined = $state(undefined)

	// Create snapshot form
	let newSnapshotName = $state('')
	let newSnapshotTag = $state('latest')
	let newSnapshotDockerImage = $state('')
	let newSnapshotSetupScript = $state('')

	// Create volume form
	let newVolumeName = $state('')

	// Upload state
	let uploadDrawer: Drawer | undefined = $state()
	let uploadName = $state('')
	let uploadTag = $state('latest')
	let uploadFile: File | null = $state(null)
	let uploading = $state(false)

	// Instructions drawer
	let instructionsDrawer: Drawer | undefined = $state()

	async function apiFetch(path: string, options?: RequestInit) {
		const resp = await fetch(`/api/w/${$workspaceStore}/sandbox${path}`, options)
		if (!resp.ok) {
			const text = await resp.text()
			throw new Error(text || resp.statusText)
		}
		return resp
	}

	async function loadSnapshots() {
		try {
			const resp = await apiFetch('/snapshots')
			snapshots = await resp.json()
		} catch (e: any) {
			sendUserToast(`Failed to load snapshots: ${e.message}`, true)
			snapshots = []
		}
	}

	async function loadVolumes() {
		try {
			const resp = await apiFetch('/volumes')
			volumes = await resp.json()
		} catch (e: any) {
			sendUserToast(`Failed to load volumes: ${e.message}`, true)
			volumes = []
		}
	}

	async function createSnapshot(close: () => void) {
		try {
			await apiFetch('/snapshots', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					name: newSnapshotName,
					tag: newSnapshotTag || 'latest',
					docker_image: newSnapshotDockerImage,
					setup_script: newSnapshotSetupScript || null
				})
			})
			sendUserToast(`Snapshot ${newSnapshotName}:${newSnapshotTag || 'latest'} created`)
			newSnapshotName = ''
			newSnapshotTag = 'latest'
			newSnapshotDockerImage = ''
			newSnapshotSetupScript = ''
			close()
			loadSnapshots()
		} catch (e: any) {
			sendUserToast(`Failed to create snapshot: ${e.message}`, true)
		}
	}

	async function deleteSnapshot(name: string, tag: string) {
		try {
			await apiFetch(`/snapshots/${encodeURIComponent(name)}/${encodeURIComponent(tag)}`, {
				method: 'DELETE'
			})
			sendUserToast(`Deleted snapshot ${name}:${tag}`)
			loadSnapshots()
		} catch (e: any) {
			sendUserToast(`Failed to delete snapshot: ${e.message}`, true)
		}
	}

	async function rebuildSnapshot(name: string, tag: string) {
		try {
			await apiFetch(
				`/snapshots/${encodeURIComponent(name)}/${encodeURIComponent(tag)}/rebuild`,
				{ method: 'POST' }
			)
			sendUserToast(`Rebuild queued for ${name}:${tag}`)
			loadSnapshots()
		} catch (e: any) {
			sendUserToast(`Failed to rebuild snapshot: ${e.message}`, true)
		}
	}

	async function uploadSnapshot() {
		if (!uploadFile || !uploadName) return
		uploading = true
		try {
			const resp = await fetch(
				`/api/w/${$workspaceStore}/sandbox/snapshots/${encodeURIComponent(uploadName)}/${encodeURIComponent(uploadTag || 'latest')}/upload`,
				{
					method: 'POST',
					headers: { 'Content-Type': 'application/octet-stream' },
					body: uploadFile
				}
			)
			if (!resp.ok) {
				throw new Error((await resp.text()) || resp.statusText)
			}
			sendUserToast(`Snapshot ${uploadName}:${uploadTag || 'latest'} uploaded successfully`)
			uploadName = ''
			uploadTag = 'latest'
			uploadFile = null
			uploadDrawer?.closeDrawer()
			loadSnapshots()
		} catch (e: any) {
			sendUserToast(`Upload failed: ${e.message}`, true)
		} finally {
			uploading = false
		}
	}

	async function createVolume(close: () => void) {
		try {
			await apiFetch('/volumes', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ name: newVolumeName })
			})
			sendUserToast(`Volume ${newVolumeName} created`)
			newVolumeName = ''
			close()
			loadVolumes()
		} catch (e: any) {
			sendUserToast(`Failed to create volume: ${e.message}`, true)
		}
	}

	async function deleteVolume(name: string) {
		try {
			await apiFetch(`/volumes/${encodeURIComponent(name)}`, { method: 'DELETE' })
			sendUserToast(`Deleted volume ${name}`)
			loadVolumes()
		} catch (e: any) {
			sendUserToast(`Failed to delete volume: ${e.message}`, true)
		}
	}

	function formatSize(bytes: number | null): string {
		if (bytes == null) return '-'
		if (bytes < 1024) return `${bytes} B`
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
		return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
	}

	function statusColor(
		status: string
	): 'green' | 'yellow' | 'red' | 'blue' | 'gray' {
		switch (status) {
			case 'ready':
				return 'green'
			case 'building':
				return 'blue'
			case 'pending':
				return 'yellow'
			case 'failed':
				return 'red'
			default:
				return 'gray'
		}
	}

	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				loadSnapshots()
				loadVolumes()
			})
		}
	})
</script>

<Drawer bind:this={uploadDrawer}>
	<DrawerContent title="Upload Snapshot" on:close={uploadDrawer?.closeDrawer}>
		<div class="flex flex-col gap-4">
			<p class="text-secondary text-xs">
				Upload a pre-built rootfs tar.gz file to use as a sandbox snapshot. You can create one
				using:
			</p>
			<pre
				class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto"
			>crane export python:3.11-slim - | gzip &gt; snapshot.tar.gz</pre>
			<label class="block">
				<span class="text-xs font-semibold">Name</span>
				<input
					class="w-full mt-1"
					bind:value={uploadName}
					placeholder="e.g. python-env"
				/>
			</label>
			<label class="block">
				<span class="text-xs font-semibold">Tag</span>
				<input class="w-full mt-1" bind:value={uploadTag} placeholder="latest" />
			</label>
			<label class="block">
				<span class="text-xs font-semibold">Snapshot file (.tar.gz)</span>
				<input
					type="file"
					accept=".tar.gz,.tgz"
					class="w-full mt-1"
					onchange={(e) => {
						const target = e.target as HTMLInputElement
						uploadFile = target.files?.[0] ?? null
					}}
				/>
			</label>
			{#if uploadFile}
				<p class="text-xs text-secondary">
					File: {uploadFile.name} ({formatSize(uploadFile.size)})
				</p>
			{/if}
			<Button
				variant="accent"
				disabled={!uploadName || !uploadFile || uploading}
				on:click={uploadSnapshot}
			>
				{uploading ? 'Uploading...' : 'Upload'}
			</Button>
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={instructionsDrawer}>
	<DrawerContent title="How to use Sandbox Snapshots & Volumes" on:close={instructionsDrawer?.closeDrawer}>
		<div class="flex flex-col gap-6 text-sm">
			<section>
				<h3 class="font-semibold text-base mb-2">Overview</h3>
				<p class="text-secondary">
					Sandbox snapshots provide custom rootfs environments for nsjail-sandboxed scripts.
					Volumes provide persistent storage that syncs to/from S3 between job runs.
				</p>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">1. Create a Snapshot</h3>
				<p class="text-secondary mb-2">Export a Docker image as a tar.gz rootfs:</p>
				<pre class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto">
# Install crane (Go required)
go install github.com/google/go-containerregistry/cmd/crane@latest

# Export a Docker image to tar.gz
crane export python:3.11-slim - | gzip &gt; python-slim.tar.gz

# Or use docker directly
docker create --name tmp python:3.11-slim
docker export tmp | gzip &gt; python-slim.tar.gz
docker rm tmp</pre>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">2. Upload the Snapshot</h3>
				<p class="text-secondary">
					Use the "Upload Snapshot" button to upload the tar.gz file. It will be stored in your
					configured S3 object store and marked as "ready".
				</p>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">3. Use in Scripts</h3>
				<p class="text-secondary mb-2">
					Reference snapshots and volumes using comment annotations in your script:
				</p>
				<pre class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto">
# Python / Bash
# sandbox: python-env:latest
# volume: data:/workspace/data

def main():
    import pandas as pd  # available from snapshot
    # /workspace/data persists between runs</pre>
				<pre class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto mt-2">
// TypeScript / Go
// sandbox: node-env:v2
// volume: cache:/tmp/cache

export async function main() &#123;
    // Custom Node.js environment from snapshot
&#125;</pre>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">Annotation Reference</h3>
				<div class="border rounded-md overflow-hidden">
					<table class="w-full text-xs">
						<thead class="bg-surface-secondary">
							<tr>
								<th class="text-left p-2">Annotation</th>
								<th class="text-left p-2">Description</th>
							</tr>
						</thead>
						<tbody>
							<tr class="border-t">
								<td class="p-2 font-mono"># sandbox: name:tag</td>
								<td class="p-2">Use snapshot "name" with "tag" as rootfs (tag defaults to "latest")</td>
							</tr>
							<tr class="border-t">
								<td class="p-2 font-mono"># volume: name:/mount/path</td>
								<td class="p-2">Mount volume "name" at the given path (auto-created if new)</td>
							</tr>
						</tbody>
					</table>
				</div>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">Requirements</h3>
				<ul class="list-disc ml-4 text-secondary space-y-1">
					<li>S3 object store must be configured in Instance Settings</li>
					<li>nsjail must be available on workers</li>
					<li>Workers need overlayfs support (root or fuse-overlayfs) for snapshots</li>
					<li>Volumes work without overlayfs — they are simple bind mounts</li>
				</ul>
			</section>
		</div>
	</DrawerContent>
</Drawer>

{#if !$userStore?.is_admin && !$userStore?.is_super_admin}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Sandbox management requires admin permissions</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Sandboxes"
			tooltip="Manage sandbox snapshots (custom rootfs environments) and volumes (persistent storage) for nsjail-sandboxed script execution."
		>
			<div class="flex flex-row gap-2">
				<Button
					variant="border"
					unifiedSize="md"
					startIcon={{ icon: Info }}
					on:click={() => instructionsDrawer?.openDrawer()}
				>
					How to use
				</Button>
				{#if tab === 'snapshots'}
					<Button
						variant="border"
						unifiedSize="md"
						startIcon={{ icon: Upload }}
						on:click={() => uploadDrawer?.openDrawer()}
					>
						Upload snapshot
					</Button>
					<Popover
						floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
						contentClasses="flex flex-col gap-2 p-4 w-80"
					>
						{#snippet trigger()}
							<Button variant="accent" unifiedSize="md" startIcon={{ icon: Plus }} nonCaptureEvent>
								New snapshot
							</Button>
						{/snippet}
						{#snippet content({ close })}
							<label class="block">
								<span class="text-xs font-semibold">Name</span>
								<input
									class="w-full mt-1"
									bind:value={newSnapshotName}
									placeholder="e.g. python-env"
								/>
							</label>
							<label class="block">
								<span class="text-xs font-semibold">Tag</span>
								<input
									class="w-full mt-1"
									bind:value={newSnapshotTag}
									placeholder="latest"
								/>
							</label>
							<label class="block">
								<span class="text-xs font-semibold">Docker image</span>
								<input
									class="w-full mt-1"
									bind:value={newSnapshotDockerImage}
									placeholder="e.g. python:3.11-slim"
								/>
							</label>
							<label class="block">
								<span class="text-xs font-semibold">Setup script (optional)</span>
								<textarea
									class="w-full mt-1 text-xs"
									rows="3"
									bind:value={newSnapshotSetupScript}
									placeholder="pip install numpy pandas"
								></textarea>
							</label>
							<Button
								variant="accent"
								startIcon={{ icon: Plus }}
								disabled={!newSnapshotName || !newSnapshotDockerImage}
								on:click={() => createSnapshot(close)}
							>
								Create
							</Button>
						{/snippet}
					</Popover>
				{:else}
					<Popover
						floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
						contentClasses="flex flex-col gap-2 p-4 w-72"
					>
						{#snippet trigger()}
							<Button variant="accent" unifiedSize="md" startIcon={{ icon: Plus }} nonCaptureEvent>
								New volume
							</Button>
						{/snippet}
						{#snippet content({ close })}
							<label class="block">
								<span class="text-xs font-semibold">Volume name</span>
								<input
									class="w-full mt-1"
									bind:value={newVolumeName}
									placeholder="e.g. shared-data"
									onkeyup={(e) => {
										if (e.key === 'Enter') createVolume(close)
									}}
								/>
							</label>
							<Button
								variant="accent"
								startIcon={{ icon: Plus }}
								disabled={!newVolumeName}
								on:click={() => createVolume(close)}
							>
								Create
							</Button>
						{/snippet}
					</Popover>
				{/if}
			</div>
		</PageHeader>

		<Tabs bind:selected={tab}>
			{#snippet children()}
				<Tab value="snapshots" label="Snapshots" />
				<Tab value="volumes" label="Volumes" />
			{/snippet}
			{#snippet content()}
				<TabContent value="snapshots">
					<div class="relative mb-20 pt-4">
						<DataTable>
							<Head>
								<tr>
									<Cell head first>Name</Cell>
									<Cell head>Tag</Cell>
									<Cell head>Status</Cell>
									<Cell head>Docker Image</Cell>
									<Cell head>Size</Cell>
									<Cell head>Created by</Cell>
									<Cell head>Updated</Cell>
									<Cell head last />
								</tr>
							</Head>
							<tbody class="divide-y">
								{#if snapshots === undefined}
									{#each new Array(3) as _}
										<tr>
											<td colspan="8"><Skeleton layout={[[2]]} /></td>
										</tr>
									{/each}
								{:else if snapshots.length === 0}
									<tr>
										<td colspan="8" class="text-secondary text-sm p-4">
											No snapshots yet. Create one from a Docker image or upload a tar.gz rootfs.
										</td>
									</tr>
								{:else}
									{#each snapshots as snapshot (snapshot.name + ':' + snapshot.tag)}
										<Row hoverable>
											<Cell first>
												<span class="text-emphasis text-xs font-semibold">{snapshot.name}</span>
											</Cell>
											<Cell>
												<span class="text-xs font-mono">{snapshot.tag}</span>
											</Cell>
											<Cell>
												<Badge color={statusColor(snapshot.status)} small>
													{snapshot.status}
												</Badge>
												{#if snapshot.build_error}
													<span class="text-red-500 text-2xs ml-1" title={snapshot.build_error}>
														(error)
													</span>
												{/if}
											</Cell>
											<Cell>
												<span class="text-xs font-mono">{snapshot.docker_image}</span>
											</Cell>
											<Cell>
												<span class="text-xs">{formatSize(snapshot.size_bytes)}</span>
											</Cell>
											<Cell>
												<span class="text-xs">{snapshot.created_by}</span>
											</Cell>
											<Cell>
												<span class="text-xs">{displayDate(snapshot.updated_at)}</span>
											</Cell>
											<Cell shouldStopPropagation>
												<Dropdown
													items={[
														{
															displayName: 'Rebuild',
															icon: RefreshCw,
															action: () =>
																rebuildSnapshot(snapshot.name, snapshot.tag)
														},
														{
															displayName: 'Delete',
															icon: Trash,
															type: 'delete',
															action: () =>
																deleteSnapshot(snapshot.name, snapshot.tag)
														}
													]}
												/>
											</Cell>
										</Row>
									{/each}
								{/if}
							</tbody>
						</DataTable>
					</div>
				</TabContent>
				<TabContent value="volumes">
					<div class="relative mb-20 pt-4">
						<DataTable>
							<Head>
								<tr>
									<Cell head first>Name</Cell>
									<Cell head>Size</Cell>
									<Cell head>S3 Key</Cell>
									<Cell head>Created by</Cell>
									<Cell head>Updated</Cell>
									<Cell head last />
								</tr>
							</Head>
							<tbody class="divide-y">
								{#if volumes === undefined}
									{#each new Array(3) as _}
										<tr>
											<td colspan="6"><Skeleton layout={[[2]]} /></td>
										</tr>
									{/each}
								{:else if volumes.length === 0}
									<tr>
										<td colspan="6" class="text-secondary text-sm p-4">
											No volumes yet. Volumes are auto-created when referenced in scripts, or you
											can create one manually.
										</td>
									</tr>
								{:else}
									{#each volumes as volume (volume.name)}
										<Row hoverable>
											<Cell first>
												<span class="text-emphasis text-xs font-semibold">{volume.name}</span>
											</Cell>
											<Cell>
												<span class="text-xs">{formatSize(volume.size_bytes)}</span>
											</Cell>
											<Cell>
												<span class="text-xs font-mono text-secondary">{volume.s3_key}</span>
											</Cell>
											<Cell>
												<span class="text-xs">{volume.created_by}</span>
											</Cell>
											<Cell>
												<span class="text-xs">{displayDate(volume.updated_at)}</span>
											</Cell>
											<Cell shouldStopPropagation>
												<Dropdown
													items={[
														{
															displayName: 'Delete',
															icon: Trash,
															type: 'delete',
															action: () => deleteVolume(volume.name)
														}
													]}
												/>
											</Cell>
										</Row>
									{/each}
								{/if}
							</tbody>
						</DataTable>
					</div>
				</TabContent>
			{/snippet}
		</Tabs>
	</CenteredPage>
{/if}
