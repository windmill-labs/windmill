<script lang="ts">
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import { Button } from '$lib/components/common'
	import Select from '../select/Select.svelte'
	import { Loader2, RotateCw, X } from 'lucide-svelte'
	import { GitSyncService, WorkspaceService, type Workspace } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Tooltip from '../Tooltip.svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	// Ensure the nested object exists
	if (!$values['github_enterprise_app']) {
		$values['github_enterprise_app'] = {}
	}

	let selfManaged = $derived(!!$values['github_enterprise_app'].self_managed)
	let fieldsDisabled = $derived(disabled || !selfManaged)

	// --- Workspace assignments ---

	type Discovered = {
		installation_id: number
		account_id: string
		assigned_workspaces: Array<{ workspace_id: string; provisioned_by_admin: boolean }>
	}

	let discovered: Discovered[] = $state([])
	let workspaces: Workspace[] = $state([])
	let loadingDiscovery = $state(false)
	let discoveryError: string | undefined = $state(undefined)
	// True once we've attempted at least one discovery call. Prevents the
	// auto-prefetch $effect from re-firing when a successful refresh
	// legitimately returns zero installations.
	let hasDiscovered = $state(false)
	// installation_id -> workspace_id picked in the row's dropdown
	let pickedWorkspace: Record<number, string | undefined> = $state({})
	// installation_id -> in-flight assign/unassign (so the row spinner shows)
	let busyInstallation: Record<number, boolean> = $state({})

	async function loadWorkspaces() {
		try {
			const perPage = 100
			const all: Workspace[] = []
			let page = 1
			// Defensive cap; an instance with >10k workspaces is implausible.
			while (page <= 100) {
				const batch = await WorkspaceService.listWorkspacesAsSuperAdmin({ page, perPage })
				all.push(...batch)
				if (batch.length < perPage) break
				page++
			}
			workspaces = all
		} catch (err: any) {
			console.error('Failed to list workspaces:', err)
			sendUserToast(
				`Failed to load workspaces for assignment: ${err?.body?.error?.message || err?.message || 'unknown error'}`,
				true
			)
		}
	}

	async function refreshDiscovery() {
		loadingDiscovery = true
		discoveryError = undefined
		hasDiscovered = true
		try {
			discovered = await GitSyncService.discoverGhesInstallations()
		} catch (err: any) {
			discoveryError = err?.body?.error?.message || err?.message || 'Failed to load installations'
			discovered = []
		} finally {
			loadingDiscovery = false
		}
	}

	function workspaceOptions(install: Discovered) {
		const assignedIds = new Set(install.assigned_workspaces.map((a) => a.workspace_id))
		return workspaces
			.filter((w) => !assignedIds.has(w.id))
			.map((w) => ({ label: `${w.name} (${w.id})`, value: w.id }))
	}

	async function assign(installation_id: number) {
		const workspace_id = pickedWorkspace[installation_id]
		if (!workspace_id) return
		busyInstallation[installation_id] = true
		try {
			await GitSyncService.assignGhesInstallation({
				requestBody: { workspace_id, installation_id }
			})
			sendUserToast(`Assigned installation to workspace ${workspace_id}`, false)
			pickedWorkspace[installation_id] = undefined
			await refreshDiscovery()
		} catch (err: any) {
			sendUserToast(
				`Failed to assign installation: ${err?.body?.error?.message || err?.message || 'unknown error'}`,
				true
			)
		} finally {
			busyInstallation[installation_id] = false
		}
	}

	async function unassign(installation_id: number, workspace_id: string) {
		busyInstallation[installation_id] = true
		try {
			await GitSyncService.unassignGhesInstallation({
				workspaceId: workspace_id,
				installationId: installation_id
			})
			sendUserToast(`Unassigned installation from workspace ${workspace_id}`, false)
			await refreshDiscovery()
		} catch (err: any) {
			sendUserToast(
				`Failed to unassign installation: ${err?.body?.error?.message || err?.message || 'unknown error'}`,
				true
			)
		} finally {
			busyInstallation[installation_id] = false
		}
	}

	let assignmentsReady = $derived(
		selfManaged &&
			!!$values['github_enterprise_app'].base_url &&
			!!$values['github_enterprise_app'].app_id &&
			!!$values['github_enterprise_app'].private_key
	)

	$effect(() => {
		if (assignmentsReady && workspaces.length === 0) {
			loadWorkspaces()
		}
		if (assignmentsReady && !hasDiscovered) {
			refreshDiscovery()
		}
	})
</script>

<div class="space-y-6">
	<Toggle
		size="xs"
		options={{ right: 'Self-managed GitHub App' }}
		checked={selfManaged}
		on:change={() => {
			$values['github_enterprise_app'] = {
				...$values['github_enterprise_app'],
				self_managed: !selfManaged
			}
		}}
	/>

	{#if !selfManaged}
		<p class="text-xs text-secondary">
			Using the managed Windmill GitHub App via stats.windmill.dev. Enable self-managed mode to
			configure your own GitHub App (required for GitHub Enterprise Server).
		</p>
	{:else}
		<details class="mt-1">
			<summary class="text-xs font-medium text-secondary cursor-pointer hover:text-primary"
				>How to create a GitHub App</summary
			>
			<div class="mt-2 p-3 bg-surface rounded text-2xs text-secondary space-y-2">
				<p>
					<strong>1.</strong> On your GitHub instance, go to
					<strong
						>Settings &rarr; Developer settings &rarr; GitHub Apps &rarr; New GitHub App</strong
					>.
				</p>
				<p><strong>2.</strong> Fill in the required fields:</p>
				<ul class="list-disc ml-4 space-y-1">
					<li>
						<strong>GitHub App name</strong>: e.g. <code>windmill-sync</code> (this becomes the app slug)
					</li>
					<li>
						<strong>Homepage URL</strong>: your Windmill instance URL
					</li>
					<li>
						<strong>Callback URL</strong>: <code>&lt;your-windmill-url&gt;/gh_success</code>
					</li>
					<li>
						<strong>Setup URL</strong> (optional):
						<code>&lt;your-windmill-url&gt;/gh_success</code> with "Redirect on update" checked
					</li>
					<li>Uncheck <strong>Active</strong> under Webhook (not needed)</li>
				</ul>
				<p><strong>3.</strong> Set repository permissions:</p>
				<ul class="list-disc ml-4 space-y-1">
					<li><strong>Contents</strong>: Read &amp; write</li>
					<li><strong>Metadata</strong>: Read-only</li>
				</ul>
				<p>
					<strong>4.</strong> Under "Where can this GitHub App be installed?", choose
					<strong>Any account</strong> (or restrict to your organization).
				</p>
				<p>
					<strong>5.</strong> Click <strong>Create GitHub App</strong>. On the next page, note the
					<strong>App ID</strong> and <strong>Client ID</strong>.
				</p>
				<p>
					<strong>6.</strong> Scroll down and click <strong>Generate a private key</strong>. Save
					the downloaded <code>.pem</code> file — paste its contents into the Private Key field below.
				</p>
				<p>
					<strong>7.</strong> The <strong>App Slug</strong> is the URL-friendly name shown in the
					app's URL (e.g. <code>github.com/apps/<strong>windmill-sync</strong></code>).
				</p>
				<p>
					<strong>8.</strong> The <strong>Base URL</strong> is your GitHub instance root (e.g.
					<code>https://github.com</code> or <code>https://github.mycompany.com</code>).
				</p>
			</div>
		</details>
	{/if}

	<div class="grid grid-cols-2 gap-x-2 gap-y-6">
		<div class="flex flex-col gap-1">
			<label for="ghes_base_url" class="block text-xs font-semibold text-emphasis mb-1">
				Base URL
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_base_url',
					placeholder: 'https://github.mycompany.com',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].base_url}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_app_id" class="block text-xs font-semibold text-emphasis mb-1">
				App ID
			</label>
			<TextInput
				inputProps={{
					type: 'number',
					id: 'ghes_app_id',
					placeholder: '12345',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].app_id}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_app_slug" class="block text-xs font-semibold text-emphasis mb-1">
				App Slug
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_app_slug',
					placeholder: 'my-windmill-app',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].app_slug}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_client_id" class="block text-xs font-semibold text-emphasis mb-1">
				Client ID
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_client_id',
					placeholder: 'Iv1.abc123',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].client_id}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_app_owner" class="block text-xs font-semibold text-emphasis mb-1">
				App owner
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_app_owner',
					placeholder: 'my-org',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].app_owner}
			/>
			<span class="text-2xs text-secondary">
				Organization or user that owns the GitHub App. Required for GHE Cloud (*.ghe.com) domains.
			</span>
		</div>
	</div>

	<div class="flex flex-col gap-1">
		<label for="ghes_private_key" class="block text-xs font-semibold text-emphasis mb-1">
			Private Key (PEM)
		</label>
		<textarea
			id="ghes_private_key"
			class="w-full h-32 font-mono text-xs p-2 border rounded resize-y {fieldsDisabled
				? 'bg-surface-disabled text-disabled cursor-not-allowed'
				: 'bg-surface text-primary'}"
			placeholder="-----BEGIN RSA PRIVATE KEY-----&#10;...&#10;-----END RSA PRIVATE KEY-----"
			disabled={fieldsDisabled}
			bind:value={$values['github_enterprise_app'].private_key}
		></textarea>
	</div>

	{#if assignmentsReady}
		<div class="border-t border-gray-200 dark:border-gray-700 pt-4">
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-semibold text-emphasis">Workspace assignments</h3>
				<Button
					size="xs"
					variant="default"
					on:click={refreshDiscovery}
					disabled={loadingDiscovery}
					startIcon={{
						icon: loadingDiscovery ? Loader2 : RotateCw,
						classes: loadingDiscovery ? 'animate-spin' : ''
					}}
				>
					Refresh
				</Button>
			</div>
			<p class="text-xs text-secondary mt-1">
				Assign installations of the configured GitHub App to specific workspaces so workspace users
				don't need GitHub permissions to set up sync. Click <strong>Refresh</strong> to load installations
				the App can see (save the config above first if you haven't).
			</p>

			{#if discoveryError}
				<p class="text-xs text-red-600 dark:text-red-400 mt-2">{discoveryError}</p>
			{:else if loadingDiscovery && discovered.length === 0}
				<div class="mt-2"><Loader2 class="w-4 h-4 animate-spin" /></div>
			{:else if discovered.length === 0}
				<p class="text-xs text-secondary mt-2">
					The configured GitHub App has no installations yet. Install it on a GitHub account, then
					click <strong>Refresh</strong>.
				</p>
			{:else}
				<table class="w-full text-sm mt-3">
					<thead>
						<tr class="text-left text-xs text-secondary">
							<th class="pb-2">
								GitHub account
								<Tooltip>
									The GitHub organization or user the App is installed on (e.g.
									<code>windmill-labs</code>). A GitHub App installation is always scoped to exactly
									one account.
								</Tooltip>
							</th>
							<th class="pb-2">Installation ID</th>
							<th class="pb-2">Assigned to</th>
							<th class="pb-2"></th>
						</tr>
					</thead>
					<tbody>
						{#each discovered as install (install.installation_id)}
							<tr class="border-t border-gray-200 dark:border-gray-700 align-top">
								<td class="py-2 pr-2">{install.account_id}</td>
								<td class="py-2 pr-2 font-mono text-xs">{install.installation_id}</td>
								<td class="py-2 pr-2">
									{#if install.assigned_workspaces.length === 0}
										<span class="text-xs text-tertiary">—</span>
									{:else}
										<div class="flex flex-wrap gap-1">
											{#each install.assigned_workspaces as assignment (assignment.workspace_id)}
												<span
													class="inline-flex items-center gap-1 text-2xs px-1.5 py-0.5 rounded {assignment.provisioned_by_admin
														? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
														: 'bg-surface-secondary text-secondary'}"
													title={assignment.provisioned_by_admin
														? 'Provisioned from instance settings'
														: 'Added by the workspace itself'}
												>
													{assignment.workspace_id}
													<button
														type="button"
														class="hover:text-red-600 dark:hover:text-red-400 disabled:opacity-50"
														disabled={busyInstallation[install.installation_id]}
														title={assignment.provisioned_by_admin
															? 'Unassign from this workspace'
															: 'Force-remove from this workspace'}
														onclick={() =>
															unassign(install.installation_id, assignment.workspace_id)}
													>
														<X class="w-3 h-3" />
													</button>
												</span>
											{/each}
										</div>
									{/if}
								</td>
								<td class="py-2">
									<div class="flex items-center gap-1 justify-end">
										<div class="w-56">
											<Select
												items={workspaceOptions(install)}
												bind:value={pickedWorkspace[install.installation_id]}
												placeholder="Pick a workspace…"
												size="sm"
												clearable
												noItemsMsg="No more workspaces to assign"
											/>
										</div>
										<Button
											size="xs"
											variant="accent"
											disabled={!pickedWorkspace[install.installation_id] ||
												busyInstallation[install.installation_id]}
											on:click={() => assign(install.installation_id)}
										>
											Assign
										</Button>
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	{/if}
</div>
