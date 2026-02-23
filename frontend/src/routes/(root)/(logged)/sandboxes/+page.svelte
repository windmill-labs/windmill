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
		Skeleton
	} from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { Plus, Trash, Upload, RefreshCw, Info, ExternalLink } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { base } from '$app/paths'
	import { displayDate } from '$lib/utils'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Select from '$lib/components/select/Select.svelte'

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

	let snapshots: Snapshot[] | undefined = $state(undefined)

	// Create snapshot drawer
	let createDrawer: Drawer | undefined = $state()
	let snapshotMode: 'builder' | 'dockerfile' | 'upload' = $state('builder')

	// Shared state
	let newName = $state('')
	let newTag = $state('latest')

	// Builder state
	let dockerImageItems = $state([
		{ label: 'Debian Slim (recommended)', value: 'debian:bookworm-slim' },
		{ label: 'Ubuntu 24.04', value: 'ubuntu:24.04' },
		{ label: 'Python 3.12 Slim', value: 'python:3.12-slim' },
		{ label: 'Node 22 Slim', value: 'node:22-slim' }
	])
	let newDockerImage: string | undefined = $state('debian:bookworm-slim')
	let newSetupScript = $state('')

	// Dockerfile state
	let dockerfileFileInput: HTMLInputElement | undefined = $state()
	let dockerfileContent = $state('')
	let dockerfileWarnings: string[] = $derived.by(() => {
		if (!dockerfileContent.trim()) return []
		return parseDockerfile(dockerfileContent).warnings
	})

	// Upload state
	let uploadFile: File | null = $state(null)
	let uploading = $state(false)

	// Detail drawer
	let detailDrawer: Drawer | undefined = $state()
	let selectedSnapshot: Snapshot | undefined = $state()

	function openSnapshotDetail(snapshot: Snapshot) {
		selectedSnapshot = snapshot
		detailDrawer?.openDrawer()
	}

	// Instructions drawer
	let instructionsDrawer: Drawer | undefined = $state()

	function parseDockerfile(content: string): {
		dockerImage: string
		setupScript: string
		warnings: string[]
	} {
		const lines = content.split('\n')
		let dockerImage = ''
		let scriptParts: string[] = []
		let warnings: string[] = []
		let continuation = ''

		for (let i = 0; i < lines.length; i++) {
			let line = lines[i].trimEnd()

			// Handle line continuations
			if (continuation) {
				line = continuation + ' ' + line.trimStart()
				continuation = ''
			}
			if (line.endsWith('\\')) {
				continuation = line.slice(0, -1).trimEnd()
				continue
			}

			const trimmed = line.trim()
			if (!trimmed || trimmed.startsWith('#')) continue

			const spaceIdx = trimmed.indexOf(' ')
			if (spaceIdx === -1) continue
			const instruction = trimmed.slice(0, spaceIdx).toUpperCase()
			const args = trimmed.slice(spaceIdx + 1).trim()

			switch (instruction) {
				case 'FROM':
					if (!dockerImage) {
						dockerImage = args.split(/\s+/)[0]
					}
					break
				case 'RUN':
					scriptParts.push(args)
					break
				case 'ENV': {
					const eqIdx = args.indexOf('=')
					if (eqIdx !== -1) {
						scriptParts.push(`export ${args}`)
					} else {
						const parts = args.split(/\s+/, 2)
						if (parts.length === 2) {
							scriptParts.push(`export ${parts[0]}=${parts[1]}`)
						}
					}
					break
				}
				case 'WORKDIR':
					scriptParts.push(`mkdir -p ${args} && cd ${args}`)
					break
				default:
					warnings.push(`Unsupported instruction: ${instruction} (line ${i + 1})`)
					break
			}
		}

		return {
			dockerImage,
			setupScript: scriptParts.join('\n'),
			warnings
		}
	}

	function resetCreateForm() {
		newName = ''
		newTag = 'latest'
		newDockerImage = 'debian:bookworm-slim'
		newSetupScript = ''
		dockerfileContent = ''
		uploadFile = null
	}

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
			if (selectedSnapshot) {
				selectedSnapshot = snapshots?.find(
					(s) => s.name === selectedSnapshot!.name && s.tag === selectedSnapshot!.tag
				)
			}
		} catch (e: any) {
			sendUserToast(`Failed to load snapshots: ${e.message}`, true)
			snapshots = []
		}
	}

	async function createSnapshot() {
		const tag = newTag || 'latest'
		try {
			if (snapshotMode === 'upload') {
				if (!uploadFile || !newName) return
				uploading = true
				const resp = await fetch(
					`/api/w/${$workspaceStore}/sandbox/snapshots/${encodeURIComponent(newName)}/${encodeURIComponent(tag)}/upload`,
					{
						method: 'POST',
						headers: { 'Content-Type': 'application/octet-stream' },
						body: uploadFile
					}
				)
				if (!resp.ok) {
					throw new Error((await resp.text()) || resp.statusText)
				}
				sendUserToast(`Snapshot ${newName}:${tag} uploaded successfully`)
			} else {
				let dockerImage = newDockerImage
				let setupScript = newSetupScript
				if (snapshotMode === 'dockerfile') {
					const parsed = parseDockerfile(dockerfileContent)
					dockerImage = parsed.dockerImage
					setupScript = parsed.setupScript
				}
				await apiFetch('/snapshots', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: newName,
						tag,
						docker_image: dockerImage,
						setup_script: setupScript || null
					})
				})
				sendUserToast(`Snapshot ${newName}:${tag} created`)
			}
			resetCreateForm()
			createDrawer?.closeDrawer()
			loadSnapshots()
		} catch (e: any) {
			sendUserToast(`Failed to create snapshot: ${e.message}`, true)
		} finally {
			uploading = false
		}
	}

	async function deleteSnapshot(name: string, tag: string) {
		try {
			await apiFetch(`/snapshots/${encodeURIComponent(name)}/${encodeURIComponent(tag)}`, {
				method: 'DELETE'
			})
			sendUserToast(`Deleted snapshot ${name}:${tag}`)
			detailDrawer?.closeDrawer()
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
			detailDrawer?.closeDrawer()
			loadSnapshots()
		} catch (e: any) {
			sendUserToast(`Failed to rebuild snapshot: ${e.message}`, true)
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
			})
		}
	})

	// Auto-refresh snapshots while any are pending or building
	$effect(() => {
		const hasPending = snapshots?.some(
			(s) => s.status === 'pending' || s.status === 'building'
		)
		if (!hasPending) return

		const interval = setInterval(() => {
			loadSnapshots()
		}, 5000)

		return () => clearInterval(interval)
	})
</script>

<Drawer bind:this={createDrawer}>
	<DrawerContent title="New Snapshot" on:close={createDrawer?.closeDrawer}>
		<div class="flex flex-col gap-4">
			<label class="block">
				<span class="text-xs font-semibold">Name</span>
				<input class="w-full mt-1" bind:value={newName} placeholder="e.g. python-env" />
			</label>
			<label class="block">
				<span class="text-xs font-semibold">Tag</span>
				<input class="w-full mt-1" bind:value={newTag} placeholder="latest" />
			</label>

			<ToggleButtonGroup bind:selected={snapshotMode}>
				{#snippet children({ item })}
					<ToggleButton value="builder" label="Builder" {item} />
					<ToggleButton value="dockerfile" label="Dockerfile" {item} />
					<ToggleButton value="upload" label="Upload" icon={Upload} {item} />
				{/snippet}
			</ToggleButtonGroup>

			{#if snapshotMode === 'builder'}
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold">Docker image</span>
					<Select
						items={dockerImageItems}
						bind:value={newDockerImage}
						placeholder="Type a custom image or select a preset"
						clearable
						createText="Use custom image"
						onCreateItem={(v) => {
							if (!dockerImageItems.some((i) => i.value === v)) {
								dockerImageItems = [...dockerImageItems, { label: v, value: v }]
							}
							newDockerImage = v
						}}
					/>
				</div>
				<label class="block">
					<span class="text-xs font-semibold">Setup script (optional)</span>
					<textarea
						class="w-full mt-1 text-xs font-mono"
						rows="4"
						bind:value={newSetupScript}
						placeholder="pip install numpy pandas"
					></textarea>
				</label>
			{:else if snapshotMode === 'dockerfile'}
				<div class="flex flex-col gap-2">
					<span class="text-xs font-semibold">Dockerfile</span>
					<textarea
						class="w-full text-xs font-mono bg-surface-secondary rounded p-3"
						rows="8"
						bind:value={dockerfileContent}
						placeholder={"FROM python:3.11-slim\nRUN pip install numpy pandas"}
					></textarea>
					<input
						type="file"
						class="hidden"
						bind:this={dockerfileFileInput}
						onchange={(e) => {
							const target = e.target as HTMLInputElement
							const file = target.files?.[0]
							if (file) {
								file.text().then((text) => {
									dockerfileContent = text
								})
							}
							target.value = ''
						}}
					/>
					<Button
						variant="border"
						size="xs"
						startIcon={{ icon: Upload }}
						on:click={() => dockerfileFileInput?.click()}
					>
						Load from file
					</Button>
					{#if dockerfileWarnings.length > 0}
						<div class="flex flex-col gap-1">
							{#each dockerfileWarnings as warning}
								<Badge color="yellow" small>{warning}</Badge>
							{/each}
						</div>
					{/if}
				</div>
			{:else}
				<p class="text-secondary text-xs">
					Upload a pre-built rootfs tar.gz file. You can create one using:
				</p>
				<pre class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto"
				>crane export python:3.11-slim - | gzip &gt; snapshot.tar.gz</pre>
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
			{/if}

			<Button
				variant="accent"
				disabled={!newName ||
					(snapshotMode === 'builder' && !newDockerImage) ||
					(snapshotMode === 'dockerfile' && !dockerfileContent.trim()) ||
					(snapshotMode === 'upload' && (!uploadFile || uploading))}
				on:click={createSnapshot}
			>
				{#if snapshotMode === 'upload' && uploading}
					Uploading...
				{:else}
					Create
				{/if}
			</Button>
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={instructionsDrawer}>
	<DrawerContent title="How to use Sandbox Snapshots" on:close={instructionsDrawer?.closeDrawer}>
		<div class="flex flex-col gap-6 text-sm">
			<section>
				<h3 class="font-semibold text-base mb-2">Overview</h3>
				<p class="text-secondary">
					Sandbox snapshots provide custom rootfs environments for nsjail-sandboxed scripts.
				</p>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">1. Create a Snapshot</h3>
				<p class="text-secondary mb-2">
					Click "New snapshot" to open the creation drawer. Three modes are available:
				</p>
				<div class="flex flex-col gap-3">
					<div>
						<p class="font-medium text-xs mb-1">Builder</p>
						<p class="text-secondary text-xs">
							Specify a Docker image (e.g. <code class="text-2xs">python:3.11-slim</code>) and an
							optional setup script. Windmill pulls the image and runs the script to build the
							snapshot on a worker.
						</p>
					</div>
					<div>
						<p class="font-medium text-xs mb-1">Dockerfile</p>
						<p class="text-secondary text-xs">
							Paste or upload a Dockerfile. <code class="text-2xs">FROM</code> sets the base image;
							<code class="text-2xs">RUN</code>, <code class="text-2xs">ENV</code>, and
							<code class="text-2xs">WORKDIR</code> are converted to a setup script.
							Unsupported instructions (COPY, ADD, etc.) show a warning but don't block creation.
						</p>
					</div>
					<div>
						<p class="font-medium text-xs mb-1">Upload</p>
						<p class="text-secondary text-xs">
							Upload a pre-built rootfs <code class="text-2xs">.tar.gz</code> file. You can create
							one with:
						</p>
						<pre class="bg-surface-secondary rounded p-2 text-xs overflow-x-auto mt-1">crane export python:3.11-slim - | gzip &gt; snapshot.tar.gz</pre>
					</div>
				</div>
			</section>

			<section>
				<h3 class="font-semibold text-base mb-2">2. Use in Scripts</h3>
				<p class="text-secondary mb-2">
					Reference snapshots using comment annotations in your script:
				</p>
				<p class="text-xs font-medium mb-1">Python / Bash</p>
				<pre class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto">
# sandbox: python-env:latest

def main():
    import pandas as pd  # available from snapshot</pre>
				<p class="text-xs font-medium mb-1 mt-2">TypeScript / Go</p>
				<pre class="bg-surface-secondary rounded p-3 text-xs overflow-x-auto">
// sandbox: node-env:v2

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
				</ul>
			</section>
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={detailDrawer}>
	<DrawerContent
		title={selectedSnapshot ? `${selectedSnapshot.name}:${selectedSnapshot.tag}` : ''}
		on:close={detailDrawer?.closeDrawer}
	>
		{#snippet actions()}
			{#if selectedSnapshot}
				<Button
					variant="border"
					size="xs"
					startIcon={{ icon: RefreshCw }}
					on:click={() => rebuildSnapshot(selectedSnapshot!.name, selectedSnapshot!.tag)}
				>
					Rebuild
				</Button>
				<Button
					color="red"
					variant="border"
					size="xs"
					startIcon={{ icon: Trash }}
					on:click={() => deleteSnapshot(selectedSnapshot!.name, selectedSnapshot!.tag)}
				>
					Delete
				</Button>
			{/if}
		{/snippet}
		{#if selectedSnapshot}
			<div class="flex flex-col gap-6 text-sm">
				<div class="grid grid-cols-[auto,1fr] gap-x-4 gap-y-2 items-baseline">
					<span class="text-secondary text-xs">Status</span>
					<Badge color={statusColor(selectedSnapshot.status)} small>
						{selectedSnapshot.status}
					</Badge>

					<span class="text-secondary text-xs">Size</span>
					<span class="text-xs">{formatSize(selectedSnapshot.size_bytes)}</span>

					<span class="text-secondary text-xs">Created</span>
					<span class="text-xs">
						{selectedSnapshot.created_by} &middot; {displayDate(selectedSnapshot.created_at)}
					</span>

					<span class="text-secondary text-xs">Updated</span>
					<span class="text-xs">{displayDate(selectedSnapshot.updated_at)}</span>
				</div>

				<section>
					<h4 class="font-semibold text-xs mb-2">Build Configuration</h4>
					<div class="grid grid-cols-[auto,1fr] gap-x-4 gap-y-2 items-baseline">
						<span class="text-secondary text-xs">Docker Image</span>
						<span class="text-xs font-mono">{selectedSnapshot.docker_image}</span>

						<span class="text-secondary text-xs">Setup Script</span>
						<div>
							{#if selectedSnapshot.setup_script}
								<pre class="bg-surface-secondary rounded p-2 text-xs font-mono whitespace-pre-wrap">{selectedSnapshot.setup_script}</pre>
							{:else}
								<span class="text-xs text-tertiary">None</span>
							{/if}
						</div>
					</div>
				</section>


				{#if selectedSnapshot.build_job_id || selectedSnapshot.build_error}
					<section>
						<h4 class="font-semibold text-xs mb-2">Build Job</h4>
						<div class="flex flex-col gap-2">
							{#if selectedSnapshot.build_job_id}
								<div class="flex items-center gap-2">
									<span class="text-secondary text-xs">Job ID</span>
									<a
										href="{base}/run/{selectedSnapshot.build_job_id}?workspace={$workspaceStore}"
										class="text-xs font-mono text-blue-500 hover:underline inline-flex items-center gap-1"
									>
										{selectedSnapshot.build_job_id}
										<ExternalLink size={12} />
									</a>
								</div>
							{/if}
							{#if selectedSnapshot.build_error}
								<div>
									<span class="text-secondary text-xs">Error</span>
									<pre class="bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded p-2 text-xs font-mono whitespace-pre-wrap mt-1">{selectedSnapshot.build_error}</pre>
								</div>
							{/if}
						</div>
					</section>
				{/if}

				<section>
					<h4 class="font-semibold text-xs mb-2">Storage</h4>
					<div class="grid grid-cols-[auto,1fr] gap-x-4 gap-y-2 items-baseline">
						<span class="text-secondary text-xs">S3 Key</span>
						<span class="text-xs font-mono text-secondary break-all">{selectedSnapshot.s3_key}</span>

						<span class="text-secondary text-xs">Content Hash</span>
						<span class="text-xs font-mono text-secondary break-all">{selectedSnapshot.content_hash || '-'}</span>
					</div>
				</section>
			</div>
		{/if}
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
			tooltip="Manage sandbox snapshots (custom rootfs environments) for nsjail-sandboxed script execution."
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
				<Button
					variant="accent"
					unifiedSize="md"
					startIcon={{ icon: Plus }}
					on:click={() => createDrawer?.openDrawer()}
				>
					New snapshot
				</Button>
			</div>
		</PageHeader>

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
							<Row hoverable on:click={() => openSnapshotDetail(snapshot)}>
								<Cell first>
									<span class="text-emphasis text-xs font-semibold">{snapshot.name}</span>
								</Cell>
								<Cell>
									<span class="text-xs font-mono">{snapshot.tag}</span>
								</Cell>
								<Cell>
									<div class="flex items-center gap-1.5">
										{#if snapshot.build_job_id}
											<a href="{base}/run/{snapshot.build_job_id}?workspace={$workspaceStore}">
												<Badge color={statusColor(snapshot.status)} small>
													{snapshot.status}
												</Badge>
											</a>
										{:else}
											<Badge color={statusColor(snapshot.status)} small>
												{snapshot.status}
											</Badge>
										{/if}
										{#if snapshot.build_error}
											<Tooltip small>
												{#snippet text()}
													<pre class="whitespace-pre-wrap text-2xs max-w-md">{snapshot.build_error}</pre>
												{/snippet}
												<span class="text-red-500 text-2xs cursor-help underline decoration-dotted">
													(error)
												</span>
											</Tooltip>
										{/if}
									</div>
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
											...(snapshot.build_job_id
												? [
														{
															displayName: 'View build job',
															icon: ExternalLink,
															href: `${base}/run/${snapshot.build_job_id}?workspace=${$workspaceStore}`
														}
													]
												: []),
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
	</CenteredPage>
{/if}
