<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import { AlertTriangle, CornerDownLeft, Download, Loader2 } from 'lucide-svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { UserService, FolderService } from '$lib/gen'
	import type { WorkspaceOffboardPreview, OffboardAffectedPaths } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'

	type Props = {
		open: boolean
		email: string
		reassignOnly?: boolean
		onClose: () => void
		onComplete: () => void
	}

	let { open = $bindable(), email, reassignOnly = false, onClose, onComplete }: Props = $props()

	let workspacePreviews: WorkspaceOffboardPreview[] = $state([])
	let loading = $state(false)
	let submitting = $state(false)
	let deleteUser = $state(true)

	$effect(() => {
		deleteUser = !reassignOnly
	})

	let wsConfigs: Record<
		string,
		{
			targetKind: 'user' | 'folder'
			selectedUser: string | undefined
			selectedFolder: string | undefined
			selectedOperator: string | undefined
			users: Array<{ label: string; value: string }>
			folders: Array<{ label: string; value: string }>
		}
	> = $state({})

	function countPaths(p: OffboardAffectedPaths | undefined | null): number {
		if (!p) return 0
		return (
			(p.scripts?.length ?? 0) +
			(p.flows?.length ?? 0) +
			(p.apps?.length ?? 0) +
			(p.resources?.length ?? 0) +
			(p.variables?.length ?? 0) +
			(p.schedules?.length ?? 0) +
			(p.triggers?.length ?? 0)
		)
	}

	let workspacesWithItems = $derived(
		workspacePreviews.filter(
			(wp) => countPaths(wp.preview.owned) > 0 || countPaths(wp.preview.executing_on_behalf) > 0
		)
	)

	$effect(() => {
		if (open) {
			loadPreview()
		}
	})

	async function loadPreview() {
		loading = true
		try {
			const result = await UserService.globalOffboardPreview({ email })
			workspacePreviews = result.workspaces

			for (const wp of result.workspaces) {
				if (countPaths(wp.preview.owned) > 0) {
					const [usernamesList, foldersList] = await Promise.all([
						UserService.listUsernames({ workspace: wp.workspace_id }),
						FolderService.listFolders({ workspace: wp.workspace_id })
					])
					wsConfigs[wp.workspace_id] = {
						targetKind: 'user',
						selectedUser: undefined,
						selectedFolder: undefined,
						selectedOperator: undefined,
						users: usernamesList
							.filter((u) => u !== wp.username)
							.map((u) => ({ label: u, value: u })),
						folders: foldersList.map((f) => ({ label: f.name, value: f.name }))
					}
				}
			}
		} catch (e) {
			sendUserToast('Failed to load offboard preview', true)
			onClose()
		} finally {
			loading = false
		}
	}

	function getReassignTo(wId: string): string | undefined {
		const cfg = wsConfigs[wId]
		if (!cfg) return undefined
		return cfg.targetKind === 'user'
			? cfg.selectedUser
				? `u/${cfg.selectedUser}`
				: undefined
			: cfg.selectedFolder
				? `f/${cfg.selectedFolder}`
				: undefined
	}

	let canSubmit = $derived(
		workspacesWithItems.every((wp) => {
			const target = getReassignTo(wp.workspace_id)
			const cfg = wsConfigs[wp.workspace_id]
			if (!target) return false
			if (cfg?.targetKind === 'folder' && !cfg.selectedOperator) return false
			return true
		})
	)

	async function submit() {
		submitting = true
		try {
			const reassignments: Record<string, { reassign_to: string; new_operator?: string }> = {}
			for (const wp of workspacesWithItems) {
				const target = getReassignTo(wp.workspace_id)
				const cfg = wsConfigs[wp.workspace_id]
				if (target) {
					reassignments[wp.workspace_id] = {
						reassign_to: target,
						new_operator: cfg?.targetKind === 'folder' ? cfg.selectedOperator : undefined
					}
				}
			}

			await UserService.offboardGlobalUser({
				email,
				requestBody: {
					reassignments,
					delete_user: deleteUser
				}
			})
			sendUserToast(
				deleteUser
					? `User ${email} offboarded successfully`
					: `Objects reassigned from ${email} successfully`
			)
			onComplete()
		} catch (e) {
			sendUserToast(`Offboarding failed: ${e}`, true)
		} finally {
			submitting = false
		}
	}

	function downloadAffectedList() {
		const lines: string[] = ['# Affected items for user: ' + email, '']

		for (const wp of workspacePreviews) {
			const ownedCount = countPaths(wp.preview.owned)
			const obCount = countPaths(wp.preview.executing_on_behalf)
			const refCount = countPaths(wp.preview.referencing)
			if (ownedCount === 0 && obCount === 0 && refCount === 0) continue

			lines.push(`## Workspace: ${wp.workspace_id} (${wp.username})`, '')

			function addPaths(title: string, paths: OffboardAffectedPaths | undefined) {
				if (!paths || countPaths(paths) === 0) return
				lines.push(`### ${title}`)
				for (const [kind, list] of Object.entries(paths)) {
					if (Array.isArray(list) && list.length > 0) {
						lines.push(`#### ${kind}`)
						for (const p of list) lines.push(`- ${p}`)
					}
				}
				lines.push('')
			}

			addPaths('Owned (will be reassigned)', wp.preview.owned)
			addPaths('Executing on behalf (will be updated)', wp.preview.executing_on_behalf)
			addPaths(
				'Referencing (content/values reference user paths — may break)',
				wp.preview.referencing
			)
			if (wp.preview.tokens > 0) lines.push(`Tokens: ${wp.preview.tokens} (will be deleted)`)
			if (wp.preview.http_triggers > 0)
				lines.push(`HTTP triggers: ${wp.preview.http_triggers} (webhook URLs will change)`)
			if (wp.preview.email_triggers > 0)
				lines.push(`Email triggers: ${wp.preview.email_triggers} (addresses will change)`)
			lines.push('')
		}

		const blob = new Blob([lines.join('\n')], { type: 'text/plain' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = `offboard-${email}.txt`
		a.click()
		URL.revokeObjectURL(url)
	}

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}

	let hasAnyWarnings = $derived(
		workspacePreviews.some((wp) => wp.preview.http_triggers > 0 || wp.preview.email_triggers > 0)
	)
</script>

{#if open}
	<div transition:fadeFast|local class="fixed top-0 bottom-0 left-0 right-0 z-[5000]" role="dialog">
		<div
			class={classNames(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				'ease-out duration-300 opacity-100'
			)}
		></div>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-2xl sm:p-6 max-h-[80vh] overflow-y-auto"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-800/50"
						>
							<AlertTriangle class="text-red-500 dark:text-red-400" />
						</div>
						<div class="ml-4 text-left flex-1">
							<h3 class="text-lg font-medium text-primary">
								{reassignOnly ? 'Reassign user items globally' : 'Offboard user globally'}
							</h3>
							<p class="text-sm text-secondary mt-1">
								{reassignOnly
									? `Reassign items owned by ${email} across all workspaces`
									: `Remove ${email} from instance and reassign their items`}
							</p>
						</div>
					</div>

					{#if loading}
						<div class="flex items-center justify-center py-8">
							<Loader2 class="animate-spin" size={24} />
							<span class="ml-2 text-sm text-secondary">Loading preview...</span>
						</div>
					{:else}
						<div class="mt-4 space-y-3">
							{#if workspacesWithItems.length === 0}
								<p class="text-sm text-secondary">
									This user has no owned items in any workspace.
								</p>
							{:else}
								<!-- Download button -->
								<div class="flex justify-end">
									<Button
										variant="subtle"
										size="xs2"
										startIcon={{ icon: Download }}
										onclick={downloadAffectedList}
									>
										Export full list
									</Button>
								</div>

								{#each workspacesWithItems as wp}
									{@const cfg = wsConfigs[wp.workspace_id]}
									{@const owned = countPaths(wp.preview.owned)}
									<div class="border border-border rounded-md p-3">
										<div class="flex items-center justify-between mb-2">
											<span class="text-sm font-medium text-primary">
												{wp.workspace_id}
												<span class="text-secondary font-normal">({wp.username})</span>
											</span>
											<span class="text-xs text-tertiary">{owned} item{owned !== 1 ? 's' : ''}</span
											>
										</div>

										<!-- Compact item summary -->
										<div class="flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-secondary mb-3">
											{#if (wp.preview.owned.scripts?.length ?? 0) > 0}<span
													>{wp.preview.owned.scripts?.length} scripts</span
												>{/if}
											{#if (wp.preview.owned.flows?.length ?? 0) > 0}<span
													>{wp.preview.owned.flows?.length} flows</span
												>{/if}
											{#if (wp.preview.owned.apps?.length ?? 0) > 0}<span
													>{wp.preview.owned.apps?.length} apps</span
												>{/if}
											{#if (wp.preview.owned.resources?.length ?? 0) > 0}<span
													>{wp.preview.owned.resources?.length} resources</span
												>{/if}
											{#if (wp.preview.owned.variables?.length ?? 0) > 0}<span
													>{wp.preview.owned.variables?.length} variables</span
												>{/if}
											{#if (wp.preview.owned.schedules?.length ?? 0) > 0}<span
													>{wp.preview.owned.schedules?.length} schedules</span
												>{/if}
											{#if (wp.preview.owned.triggers?.length ?? 0) > 0}<span
													>{wp.preview.owned.triggers?.length} triggers</span
												>{/if}
											{#if wp.preview.tokens > 0}<span>{wp.preview.tokens} tokens</span>{/if}
										</div>

										{#if cfg}
											<!-- Reassign target -->
											<div class="mb-2">
												<span class="text-xs font-medium text-secondary block mb-1"
													>Reassign to</span
												>
												<div class="flex items-center gap-1 mb-1.5">
													<Button
														size="xs2"
														variant={cfg.targetKind === 'user' ? 'accent' : 'default'}
														onclick={() => (cfg.targetKind = 'user')}
													>
														User
													</Button>
													<Button
														size="xs2"
														variant={cfg.targetKind === 'folder' ? 'accent' : 'default'}
														onclick={() => (cfg.targetKind = 'folder')}
													>
														Folder
													</Button>
												</div>
												{#if cfg.targetKind === 'user'}
													<Select
														items={cfg.users}
														bind:value={cfg.selectedUser}
														placeholder="Select a user..."
														size="sm"
													/>
												{:else}
													<Select
														items={cfg.folders}
														bind:value={cfg.selectedFolder}
														placeholder="Select a folder..."
														size="sm"
													/>
													<div class="mt-1">
														<span class="text-xs text-secondary block mb-0.5"
															>Run schedules/triggers as</span
														>
														<Select
															items={cfg.users}
															bind:value={cfg.selectedOperator}
															placeholder="Select operator user..."
															size="sm"
														/>
													</div>
												{/if}
											</div>
										{/if}
									</div>
								{/each}

								<!-- Global warnings -->
								{#if hasAnyWarnings}
									{@const totalHttp = workspacePreviews.reduce(
										(s, wp) => s + wp.preview.http_triggers,
										0
									)}
									{@const totalEmail = workspacePreviews.reduce(
										(s, wp) => s + wp.preview.email_triggers,
										0
									)}
									{#if totalHttp > 0 || totalEmail > 0}
										<Alert type="warning" title="Webhook and email trigger URLs will change">
											<p class="text-xs">
												{#if totalHttp > 0}{totalHttp} HTTP trigger(s) will have new webhook URLs.{/if}
												{#if totalEmail > 0}{totalEmail} email trigger(s) will have new addresses.{/if}
												Update any external integrations that reference these endpoints.
											</p>
										</Alert>
									{/if}
								{/if}
							{/if}

							<!-- Workspaces without objects -->
							{#if workspacePreviews.length > workspacesWithItems.length}
								<p class="text-xs text-tertiary">
									{workspacePreviews.length - workspacesWithItems.length} workspace(s) with no items
									to reassign
								</p>
							{/if}

							<!-- Delete user toggle -->
							{#if !reassignOnly}
								<div class="flex items-center gap-2 pt-1">
									<Toggle bind:checked={deleteUser} size="xs" />
									<span class="text-sm text-secondary">Also remove user from instance</span>
								</div>
							{/if}
						</div>
					{/if}

					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						{#if workspacesWithItems.length > 0}
							<Button
								disabled={submitting || !canSubmit}
								onclick={submit}
								variant="accent"
								size="sm"
								destructive
								shortCut={{ Icon: CornerDownLeft, hide: false, withoutModifier: true }}
							>
								{#if submitting}
									<Loader2 class="animate-spin" />
								{/if}
								<span class="min-w-20">
									{deleteUser ? 'Offboard' : 'Reassign'}
								</span>
							</Button>
						{:else if !loading}
							<Button onclick={onClose} variant="accent" size="sm">Close</Button>
						{/if}
						<Button
							disabled={submitting}
							onclick={onClose}
							variant="default"
							size="sm"
							shortCut={{ key: 'Esc', hide: false, withoutModifier: true }}
						>
							Cancel
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
