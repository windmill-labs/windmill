<script lang="ts">
	import { onMount } from 'svelte'
	import JSZip from 'jszip'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import { Download, Upload, RefreshCw } from 'lucide-svelte'

	type Listing = {
		paths: string[]
		sizes: Record<string, number>
		version: number
		edited_at: string
		edited_by: string
	}

	let listing: Listing | undefined = $state(undefined)
	let loading: boolean = $state(false)
	let uploading: boolean = $state(false)
	let fileInput: HTMLInputElement | undefined = $state(undefined)

	const isAdmin = $derived(!!$userStore?.is_admin)

	async function loadListing() {
		if (!$workspaceStore) return
		loading = true
		try {
			listing = (await WorkspaceService.listSharedUi({
				workspace: $workspaceStore
			})) as Listing
		} catch (e) {
			sendUserToast(`Failed to load shared UI: ${e}`, true)
		} finally {
			loading = false
		}
	}

	function humanSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
		return `${(bytes / 1024 / 1024).toFixed(1)} MB`
	}

	async function downloadZip() {
		if (!$workspaceStore) return
		try {
			const got = await WorkspaceService.getSharedUi({ workspace: $workspaceStore })
			const files = (got as any).files ?? {}
			const zip = new JSZip()
			for (const [path, content] of Object.entries(files)) {
				zip.file(path, content as string)
			}
			const blob = await zip.generateAsync({ type: 'blob' })
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `${$workspaceStore}-ui.zip`
			a.click()
			URL.revokeObjectURL(url)
		} catch (e) {
			sendUserToast(`Failed to download shared UI: ${e}`, true)
		}
	}

	async function onFileSelected(event: Event) {
		const target = event.target as HTMLInputElement
		const file = target.files?.[0]
		if (!file || !$workspaceStore) return
		uploading = true
		try {
			const zip = await JSZip.loadAsync(await file.arrayBuffer())
			const files: Record<string, string> = {}
			const tasks: Promise<void>[] = []
			zip.forEach((relativePath, entry) => {
				if (entry.dir) return
				tasks.push(
					entry.async('string').then((content) => {
						files[relativePath] = content
					})
				)
			})
			await Promise.all(tasks)
			await WorkspaceService.updateSharedUi({
				workspace: $workspaceStore,
				requestBody: { files }
			})
			sendUserToast(`Replaced shared UI folder with ${Object.keys(files).length} file(s)`)
			await loadListing()
		} catch (e) {
			sendUserToast(`Failed to replace shared UI: ${e}`, true)
		} finally {
			uploading = false
			if (fileInput) {
				fileInput.value = ''
			}
		}
	}

	onMount(() => {
		loadListing()
	})
</script>

<div class="mb-6">
	<h2 class="text-lg font-semibold mb-1">Shared UI folder</h2>
	<p class="text-sm text-secondary">
		Workspace-shared frontend files (components, styles, helpers) that the raw app bundler merges
		under the <code>/ui/</code> path. Raw apps can import shared components with
		<code>{`import { Button } from '/ui/Button'`}</code>.
	</p>
	<p class="text-sm text-secondary mt-2">
		Editing happens through the Windmill CLI (<code>wmill sync</code> writes/reads the
		<code>ui/</code> folder at the root of your sync directory). This page lets you inspect or
		replace the entire folder. Changes do <strong>not</strong> retroactively rebuild deployed raw apps
		— re-push affected raw apps to pick up updates.
	</p>
</div>

<div class="flex items-center gap-2 mb-4">
	<Button
		size="xs"
		variant="default"
		startIcon={{ icon: RefreshCw }}
		on:click={loadListing}
		disabled={loading}
	>
		Refresh
	</Button>
	<Button
		size="xs"
		variant="default"
		startIcon={{ icon: Download }}
		on:click={downloadZip}
		disabled={!listing || listing.paths.length === 0}
	>
		Download as zip
	</Button>
	{#if isAdmin}
		<Button
			size="xs"
			variant="default"
			startIcon={{ icon: Upload }}
			on:click={() => fileInput?.click()}
			disabled={uploading}
		>
			{uploading ? 'Replacing…' : 'Replace from zip'}
		</Button>
		<input
			bind:this={fileInput}
			type="file"
			accept=".zip"
			style="display: none;"
			onchange={onFileSelected}
		/>
	{/if}
</div>

{#if listing}
	<div class="text-xs text-secondary mb-2">
		Version {listing.version}{#if listing.edited_by}
			· Last edited by <code>{listing.edited_by}</code>
		{/if}
	</div>
	{#if listing.paths.length === 0}
		<div class="rounded border border-dashed p-6 text-center text-sm text-secondary">
			The shared UI folder is empty. Create a <code>ui/</code> directory next to your
			<code>f/</code> and <code>u/</code> folders, add files, then run <code>wmill sync push</code>.
		</div>
	{:else}
		<div class="rounded border divide-y">
			{#each listing.paths as p (p)}
				<div class="flex items-center justify-between px-3 py-2 text-sm font-mono">
					<span class="truncate">{p}</span>
					<span class="text-xs text-secondary">{humanSize(listing.sizes[p] ?? 0)}</span>
				</div>
			{/each}
		</div>
	{/if}
{:else if loading}
	<div class="text-sm text-secondary">Loading…</div>
{/if}
