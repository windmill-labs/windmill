<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Alert } from '$lib/components/common'
	import { Loader2, Eye, Save } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'
	import { JobService } from '$lib/gen'
	import { tryEvery } from '$lib/utils'

	type Settings = {
		includes: string[]
		typeFilters: { [type: string]: boolean }
		excludes: { [type: string]: boolean }
	}

	let { gitRepoResourcePath, branchName, settings } = $props<{
		gitRepoResourcePath: string
		branchName?: string
		settings: Settings
	}>()

	let _branchName = $state(branchName ?? 'main')
	let previewResult = $state<
		| {
				added: string[]
				deleted: string[]
				modified: string[]
		  }
		| undefined
	>(undefined)
	let isPreviewLoading = $state(false)
	let isInitializing = $state(false)
	let initResult = $state<{ success: boolean; message: string } | null>(null)
	let initGitRepoPopover: { open: () => void; close: () => void } | null = null
	let showYaml = $state(false)
	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)

	function toYaml(settings: Settings) {
		let yaml = '\n'
		yaml += 'includes:\n'
		for (const inc of settings.includes) {
			yaml += `  - ${inc}\n`
		}
		for (const [key, value] of Object.entries(settings.typeFilters)) {
			yaml += `skip${key.charAt(0).toUpperCase() + key.slice(1)}: ${!value}\n`
		}
		yaml += 'codebases: []\n'
		const excludeKeys = Object.entries(settings.excludes)
			.filter(([_, v]) => v)
			.map(([k]) => k)
		if (excludeKeys.length === 0) {
			yaml += 'excludes: []\n'
		} else {
			yaml += 'excludes:\n'
			excludeKeys.forEach((key) => {
				yaml += `  - ${key}\n`
			})
		}
		return yaml
	}

	async function previewChanges() {
		console.log('Previewing changes for repo:', gitRepoResourcePath)
		isPreviewLoading = true
		previewJobId = null
		previewJobStatus = undefined
		try {
			const workspace = $workspaceStore
			if (!workspace) {
				previewResult = undefined
				isPreviewLoading = false
				return
			}
			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: gitRepoResourcePath,
				branch: _branchName,
				includes: settings.includes,
				typeFilters: settings.typeFilters,
				excludes: settings.excludes,
				dry_run: true
			}
			// 1. Start the job
			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: payloadObj
			})
			previewJobId = jobId
			previewJobStatus = 'running'

			// 2. Wait for job completion (polling)
			let jobSuccess = false
			await tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: jobId
					})
					jobSuccess = !!testResult.success
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: {
								reason: 'Preview job timed out after 5s'
							}
						})
					} catch (err) {
						console.error(err)
					}
				},
				interval: 500,
				timeout: 5000
			})

			if (jobSuccess) {
				const result = await JobService.getCompletedJobResult({
					workspace,
					id: jobId
				})
				console.log('Preview result:', result)
				previewResult = result as { added: string[]; deleted: string[]; modified: string[] }
				previewJobStatus = 'success'
			} else {
				previewResult = undefined
				previewJobStatus = 'failure'
			}
		} catch (error) {
			console.error('Failed to preview changes:', error)
			previewResult = undefined
			previewJobStatus = 'failure'
		} finally {
			isPreviewLoading = false
		}
	}

	async function initializeRepo() {
		const workspace = $workspaceStore
		console.log('Initializing repo:', gitRepoResourcePath, 'in workspace:', workspace)
		isInitializing = true
		initResult = null
		try {
			const jobId = await JobService.runScriptByPath({
				workspace: workspace!,
				path: hubPaths.gitInitRepo,
				requestBody: {
					workspace_id: workspace!,
					repo_url_resource_path: gitRepoResourcePath,
					branch: _branchName,
					includes: settings.includes,
					typeFilters: settings.typeFilters,
					excludes: settings.excludes
				}
			})
			initResult = { success: true, message: `Job started: ${jobId}` }
		} catch (error) {
			console.error('Failed to initialize repo:', error)
			initResult = { success: false, message: 'Failed to initialize repository.' }
		} finally {
			isInitializing = false
		}
	}
</script>

<Popover
	bind:this={initGitRepoPopover}
	floatingConfig={{
		placement: 'bottom',
		strategy: 'fixed'
	}}
	contentClasses="p-4 w-[600px]"
>
	<svelte:fragment slot="trigger">
		<Button size="sm" nonCaptureEvent onclick={initGitRepoPopover?.open}>
			Initialize Git Repo
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<h3 class="text-lg font-semibold">Initialize Git Repository</h3>
				<Alert size="xs" type="warning" title="Warning">
					This will overwrite everything in the selected repository folder/branch.
				</Alert>
			</div>

			<div class="flex flex-col gap-2">
				<label class="text-sm font-medium">Branch Name</label>
				<input
					type="text"
					bind:value={_branchName}
					class="border rounded px-2 py-1"
					placeholder="main"
				/>
			</div>

			{#if previewResult}
				<div class="border rounded p-2 text-xs max-h-40 overflow-y-auto bg-surface-secondary">
					<div class="font-semibold text-[11px] mb-1 text-tertiary">Preview of changes:</div>
					{#if previewResult.added.length > 0}
						<div class="text-green-600">
							Added:
							<ul class="ml-4 list-disc">
								{#each previewResult.added as file}
									<li>{file}</li>
								{/each}
							</ul>
						</div>
					{/if}
					{#if previewResult.deleted.length > 0}
						<div class="text-red-600">
							Deleted:
							<ul class="ml-4 list-disc">
								{#each previewResult.deleted as file}
									<li>{file}</li>
								{/each}
							</ul>
						</div>
					{/if}
					{#if previewResult.modified.length > 0}
						<div class="text-yellow-600">
							Modified:
							<ul class="ml-4 list-disc">
								{#each previewResult.modified as file}
									<li>{file}</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			{/if}

			{#if previewJobId}
				<div class="flex items-center gap-2 text-xs text-tertiary">
					{#if previewJobStatus === 'running'}
						<svg
							class="animate-spin"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							><circle cx="12" cy="12" r="10" stroke-opacity="0.25" /><path
								d="M22 12a10 10 0 0 1-10 10"
							/></svg
						>
					{:else if previewJobStatus === 'success'}
						<svg
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							class="text-green-600"
							><path d="M9 12l2 2l4 -4" /><circle cx="12" cy="12" r="10" /></svg
						>
					{:else if previewJobStatus === 'failure'}
						<svg
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							class="text-red-700"
							><circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line
								x1="9"
								y1="9"
								x2="15"
								y2="15"
							/></svg
						>
					{/if}
					Preview job:
					<a
						target="_blank"
						class="underline"
						href={`/run/${previewJobId}?workspace=${$workspaceStore}`}
					>
						{previewJobId}
					</a>
				</div>
			{/if}

			<!-- Collapsible wmill.yaml preview section -->
			<div class="mt-2">
				<button
					type="button"
					class="flex items-center gap-1 select-none text-xs text-tertiary hover:text-primary"
					onclick={() => (showYaml = !showYaml)}
					aria-expanded={showYaml}
				>
					<svg
						class={showYaml ? 'rotate-90 transition-transform' : 'transition-transform'}
						width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
					><polyline points="9 18 15 12 9 6" /></svg>
					<span>{showYaml ? 'Hide' : 'Show'} wmill.yaml</span>
				</button>
				{#if showYaml}
					<pre class="bg-surface-secondary rounded p-2 text-2xs overflow-auto max-h-40 border mt-1">
						<code>{toYaml(settings)}</code>
					</pre>
				{/if}
			</div>

			<!-- Action row: Cancel on left, Preview/Confirm on right -->
			<div class="flex justify-between items-center mt-4">
				<Button
					color="light"
					size="xs"
					on:click={() => close()}
					disabled={isPreviewLoading || isInitializing}
				>
					Cancel
				</Button>
				<div class="flex gap-2">
					{#if !previewResult}
						<Button
							size="xs"
							on:click={previewChanges}
							disabled={isPreviewLoading || isInitializing}
							startIcon={{
								icon: isPreviewLoading ? Loader2 : Eye,
								classes: isPreviewLoading ? 'animate-spin' : ''
							}}
						>
							Preview
						</Button>
					{:else}
						<Button
							size="xs"
							on:click={previewChanges}
							disabled={isPreviewLoading || isInitializing}
							startIcon={{
								icon: isPreviewLoading ? Loader2 : Eye,
								classes: isPreviewLoading ? 'animate-spin' : ''
							}}
							title="Preview changes again"
						>
							Preview
						</Button>
						<Button
							color="red"
							size="xs"
							on:click={initializeRepo}
							disabled={isPreviewLoading || isInitializing}
							startIcon={{ icon: Save }}
							title="Initialize Git Repo"
						>
							Confirm
						</Button>
					{/if}
				</div>
			</div>

			{#if initResult}
				<div class="mt-2">
					<Alert
						type={initResult.success ? 'success' : 'error'}
						title={initResult.success ? 'Success' : 'Error'}
						size="xs"
					>
						{initResult.message}
					</Alert>
				</div>
			{/if}
		</div>
	</svelte:fragment>
</Popover>
