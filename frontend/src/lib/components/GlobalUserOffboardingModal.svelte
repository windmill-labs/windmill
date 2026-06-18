<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import { AlertTriangle, CornerDownLeft, Loader2 } from 'lucide-svelte'
	import { UserService, FolderService } from '$lib/gen'
	import type { WorkspaceOffboardPreview } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'
	import OffboardWorkspaceSection from './OffboardWorkspaceSection.svelte'
	import { countPaths } from './offboarding-utils'

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
	let doReassign = $state(true)
	let deleteUser = $state(true)
	let conflicts: string[] = $state([])

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
		conflicts = []
		try {
			const result = await UserService.globalOffboardPreview({ email })
			workspacePreviews = result.workspaces

			const configPromises = result.workspaces
				.filter(
					(wp) => countPaths(wp.preview.owned) > 0 || countPaths(wp.preview.executing_on_behalf) > 0
				)
				.map(async (wp) => {
					const [usernamesList, foldersList] = await Promise.all([
						UserService.listUsernames({ workspace: wp.workspace_id }),
						FolderService.listFolders({ workspace: wp.workspace_id })
					])
					return {
						workspace_id: wp.workspace_id,
						config: {
							targetKind: 'user' as const,
							selectedUser: undefined as string | undefined,
							selectedFolder: undefined as string | undefined,
							selectedOperator: undefined as string | undefined,
							users: usernamesList
								.filter((u: string) => u !== wp.username)
								.map((u: string) => ({ label: u, value: u })),
							folders: foldersList.map((f: { name: string }) => ({
								label: f.name,
								value: f.name
							}))
						}
					}
				})
			const configs = await Promise.all(configPromises)
			for (const { workspace_id, config } of configs) {
				wsConfigs[workspace_id] = config
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
		!doReassign ||
			workspacesWithItems.every((wp) => {
				const target = getReassignTo(wp.workspace_id)
				const cfg = wsConfigs[wp.workspace_id]
				if (!target) return false
				if (!cfg?.selectedOperator) return false
				return true
			})
	)

	async function submit() {
		submitting = true
		conflicts = []
		try {
			const reassignments: Record<string, { reassign_to: string; new_on_behalf_of_user?: string }> =
				{}
			for (const wp of workspacesWithItems) {
				const target = getReassignTo(wp.workspace_id)
				const cfg = wsConfigs[wp.workspace_id]
				if (target) {
					reassignments[wp.workspace_id] = {
						reassign_to: target,
						new_on_behalf_of_user: cfg?.selectedOperator
					}
				}
			}

			const result = await UserService.offboardGlobalUser({
				email,
				requestBody: {
					reassignments,
					delete_user: deleteUser
				}
			})
			if (result.conflicts && result.conflicts.length > 0) {
				conflicts = result.conflicts
			} else {
				sendUserToast(
					deleteUser
						? `User ${email} offboarded successfully`
						: `Items reassigned from ${email} successfully`
				)
				onComplete()
			}
		} catch (e) {
			sendUserToast(`Offboarding failed: ${e}`, true)
		} finally {
			submitting = false
		}
	}

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
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
								{#if !reassignOnly}
									<Toggle
										bind:checked={doReassign}
										size="xs"
										options={{ right: 'Reassign items before removing' }}
									/>
								{/if}

								{#if doReassign}
									{#each workspacesWithItems as wp}
										{@const cfg = wsConfigs[wp.workspace_id]}
										<div class="border border-border rounded-md p-3 space-y-2">
											<div class="flex items-center justify-between">
												<span class="text-sm font-medium text-primary">
													{wp.workspace_id}
													<span class="text-secondary font-normal">({wp.username})</span>
												</span>
											</div>

											{#if cfg}
												<OffboardWorkspaceSection
													preview={wp.preview}
													username={wp.username}
													{deleteUser}
													bind:targetKind={cfg.targetKind}
													bind:selectedUser={cfg.selectedUser}
													bind:selectedFolder={cfg.selectedFolder}
													bind:selectedOperator={cfg.selectedOperator}
													users={cfg.users}
													folders={cfg.folders}
													size="sm"
													csvFilename="offboard-{email}-{wp.workspace_id}.csv"
													instanceLevel
												/>
											{/if}
										</div>
									{/each}
								{:else}
									<Alert type="warning" title="Items will not be reassigned">
										<p class="text-xs">
											All items across {workspacesWithItems.length} workspace(s) will be left as-is.
											Triggers and runnables may stop working if the user is removed.
										</p>
									</Alert>
								{/if}
							{/if}

							{#if deleteUser}
								<Alert type="warning" title="All tokens will be deleted">
									<p class="text-xs">
										All tokens for {email} will be permanently deleted across all workspaces, including
										non-workspace-scoped tokens. This may break any API calls using these credentials.
									</p>
								</Alert>
							{/if}

							{#if workspacePreviews.length > workspacesWithItems.length}
								<p class="text-xs text-tertiary">
									{workspacePreviews.length - workspacesWithItems.length} workspace(s) with no items
									to reassign
								</p>
							{/if}

							{#if conflicts.length > 0}
								<Alert type="error" title="Path conflicts detected">
									<p class="text-xs mb-1"
										>These items already exist at the target. Rename or delete them, or choose a
										different user/folder.</p
									>
									<ul class="text-xs list-disc list-inside max-h-32 overflow-y-auto">
										{#each conflicts as conflict}
											<li>{conflict}</li>
										{/each}
									</ul>
								</Alert>
							{/if}
						</div>
					{/if}

					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						{#if workspacesWithItems.length > 0 || deleteUser}
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
