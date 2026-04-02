<script lang="ts">
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import { AlertTriangle, CornerDownLeft, Loader2 } from 'lucide-svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { UserService, FolderService, GroupService } from '$lib/gen'
	import type { WorkspaceOffboardPreview } from '$lib/gen'
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

	// Per-workspace reassignment config
	let wsConfigs: Record<
		string,
		{
			targetKind: 'user' | 'folder'
			selectedUser: string | undefined
			selectedFolder: string | undefined
			selectedOperator: string | undefined
			tokenAction: 'revoke' | 'reassign'
			tokenTargetKind: 'user' | 'group'
			selectedTokenUser: string | undefined
			selectedTokenGroup: string | undefined
			users: Array<{ label: string; value: string }>
			folders: Array<{ label: string; value: string }>
			groups: Array<{ label: string; value: string }>
		}
	> = $state({})

	let workspacesWithObjects = $derived(
		workspacePreviews.filter(
			(wp) =>
				wp.preview.scripts > 0 ||
				wp.preview.flows > 0 ||
				wp.preview.apps > 0 ||
				wp.preview.resources > 0 ||
				wp.preview.variables > 0 ||
				wp.preview.schedules > 0 ||
				wp.preview.triggers > 0 ||
				wp.preview.tokens > 0
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

			// Load users/folders/groups for each workspace with objects
			for (const wp of result.workspaces) {
				const hasObj =
					wp.preview.scripts > 0 ||
					wp.preview.flows > 0 ||
					wp.preview.apps > 0 ||
					wp.preview.resources > 0 ||
					wp.preview.variables > 0 ||
					wp.preview.schedules > 0 ||
					wp.preview.triggers > 0 ||
					wp.preview.tokens > 0

				if (hasObj) {
					const [usernamesList, foldersList, groupsList] = await Promise.all([
						UserService.listUsernames({ workspace: wp.workspace_id }),
						FolderService.listFolders({ workspace: wp.workspace_id }),
						GroupService.listGroupNames({ workspace: wp.workspace_id })
					])
					wsConfigs[wp.workspace_id] = {
						targetKind: 'user',
						selectedUser: undefined,
						selectedFolder: undefined,
						selectedOperator: undefined,
						tokenAction: 'revoke',
						tokenTargetKind: 'user',
						selectedTokenUser: undefined,
						selectedTokenGroup: undefined,
						users: usernamesList
							.filter((u) => u !== wp.username)
							.map((u) => ({ label: u, value: u })),
						folders: foldersList.map((f) => ({ label: f.name, value: f.name })),
						groups: groupsList.map((g) => ({ label: g, value: g }))
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

	function getTokenTarget(wId: string): string | undefined {
		const cfg = wsConfigs[wId]
		if (!cfg || cfg.tokenAction === 'revoke') return undefined
		return cfg.tokenTargetKind === 'user'
			? cfg.selectedTokenUser
				? `u/${cfg.selectedTokenUser}`
				: undefined
			: cfg.selectedTokenGroup
				? `g/${cfg.selectedTokenGroup}`
				: undefined
	}

	let canSubmit = $derived(
		workspacesWithObjects.every((wp) => {
			const target = getReassignTo(wp.workspace_id)
			const cfg = wsConfigs[wp.workspace_id]
			if (!target) return false
			if (cfg?.targetKind === 'folder' && !cfg.selectedOperator) return false
			if (cfg?.tokenAction === 'reassign' && !getTokenTarget(wp.workspace_id)) return false
			return true
		})
	)

	async function submit() {
		submitting = true
		try {
			const reassignments: Record<
				string,
				{ reassign_to: string; new_operator?: string; reassign_tokens_to?: string }
			> = {}
			for (const wp of workspacesWithObjects) {
				const target = getReassignTo(wp.workspace_id)
				const cfg = wsConfigs[wp.workspace_id]
				if (target) {
					reassignments[wp.workspace_id] = {
						reassign_to: target,
						new_operator: cfg?.targetKind === 'folder' ? cfg.selectedOperator : undefined,
						reassign_tokens_to: getTokenTarget(wp.workspace_id)
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

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}

	function totalObjects(p: WorkspaceOffboardPreview['preview']): number {
		return (
			p.scripts + p.flows + p.apps + p.resources + p.variables + p.schedules + p.triggers + p.tokens
		)
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
								{reassignOnly ? 'Reassign user objects globally' : 'Offboard user globally'}
							</h3>
							<p class="text-sm text-secondary mt-1">
								{reassignOnly
									? `Reassign objects owned by ${email} across all workspaces`
									: `Remove ${email} from instance and reassign their objects`}
							</p>
						</div>
					</div>

					{#if loading}
						<div class="flex items-center justify-center py-8">
							<Loader2 class="animate-spin" size={24} />
							<span class="ml-2 text-sm text-secondary">Loading preview...</span>
						</div>
					{:else}
						<div class="mt-4 space-y-4">
							{#if workspacesWithObjects.length === 0}
								<p class="text-sm text-secondary">
									This user has no owned objects in any workspace.
								</p>
							{:else}
								{#each workspacesWithObjects as wp}
									{@const cfg = wsConfigs[wp.workspace_id]}
									<div class="border border-border rounded-md p-3">
										<div class="flex items-center justify-between mb-2">
											<span class="text-sm font-medium text-primary">
												{wp.workspace_id}
												<span class="text-secondary font-normal">({wp.username})</span>
											</span>
											<span class="text-xs text-tertiary">{totalObjects(wp.preview)} objects</span>
										</div>

										<!-- Compact object summary -->
										<div class="flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-secondary mb-3">
											{#if wp.preview.scripts > 0}<span>{wp.preview.scripts} scripts</span>{/if}
											{#if wp.preview.flows > 0}<span>{wp.preview.flows} flows</span>{/if}
											{#if wp.preview.apps > 0}<span>{wp.preview.apps} apps</span>{/if}
											{#if wp.preview.resources > 0}<span>{wp.preview.resources} resources</span
												>{/if}
											{#if wp.preview.variables > 0}<span>{wp.preview.variables} variables</span
												>{/if}
											{#if wp.preview.schedules > 0}<span>{wp.preview.schedules} schedules</span
												>{/if}
											{#if wp.preview.triggers > 0}<span>{wp.preview.triggers} triggers</span>{/if}
											{#if wp.preview.tokens > 0}<span>{wp.preview.tokens} tokens</span>{/if}
										</div>

										{#if cfg}
											<!-- Reassign target -->
											<div class="mb-2">
												<label class="text-xs font-medium text-secondary block mb-1"
													>Reassign to</label
												>
												<div class="flex items-center gap-2 mb-1">
													<button
														class={classNames(
															'px-2 py-0.5 text-xs rounded border',
															cfg.targetKind === 'user'
																? 'bg-surface-selected border-border-selected text-primary'
																: 'border-border bg-surface text-secondary'
														)}
														onclick={() => (cfg.targetKind = 'user')}
													>
														User
													</button>
													<button
														class={classNames(
															'px-2 py-0.5 text-xs rounded border',
															cfg.targetKind === 'folder'
																? 'bg-surface-selected border-border-selected text-primary'
																: 'border-border bg-surface text-secondary'
														)}
														onclick={() => (cfg.targetKind = 'folder')}
													>
														Folder
													</button>
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
														<label class="text-xs text-secondary block mb-0.5"
															>Run schedules/triggers as</label
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

											<!-- Token handling (only if tokens exist) -->
											{#if wp.preview.tokens > 0}
												<div>
													<label class="text-xs font-medium text-secondary block mb-1"
														>Tokens ({wp.preview.tokens})</label
													>
													<div class="flex items-center gap-2 mb-1">
														<button
															class={classNames(
																'px-2 py-0.5 text-xs rounded border',
																cfg.tokenAction === 'revoke'
																	? 'bg-surface-selected border-border-selected text-primary'
																	: 'border-border bg-surface text-secondary'
															)}
															onclick={() => (cfg.tokenAction = 'revoke')}
														>
															Revoke
														</button>
														<button
															class={classNames(
																'px-2 py-0.5 text-xs rounded border',
																cfg.tokenAction === 'reassign'
																	? 'bg-surface-selected border-border-selected text-primary'
																	: 'border-border bg-surface text-secondary'
															)}
															onclick={() => (cfg.tokenAction = 'reassign')}
														>
															Reassign
														</button>
													</div>
													{#if cfg.tokenAction === 'reassign'}
														<div class="flex items-center gap-2 mb-1">
															<button
																class={classNames(
																	'px-2 py-0.5 text-xs rounded border',
																	cfg.tokenTargetKind === 'user'
																		? 'bg-surface-selected border-border-selected text-primary'
																		: 'border-border bg-surface text-secondary'
																)}
																onclick={() => (cfg.tokenTargetKind = 'user')}
															>
																User
															</button>
															<button
																class={classNames(
																	'px-2 py-0.5 text-xs rounded border',
																	cfg.tokenTargetKind === 'group'
																		? 'bg-surface-selected border-border-selected text-primary'
																		: 'border-border bg-surface text-secondary'
																)}
																onclick={() => (cfg.tokenTargetKind = 'group')}
															>
																Group
															</button>
														</div>
														{#if cfg.tokenTargetKind === 'user'}
															<Select
																items={cfg.users}
																bind:value={cfg.selectedTokenUser}
																placeholder="Select a user..."
																size="sm"
															/>
														{:else}
															<Select
																items={cfg.groups}
																bind:value={cfg.selectedTokenGroup}
																placeholder="Select a group..."
																size="sm"
															/>
														{/if}
													{/if}
												</div>
											{/if}
										{/if}
									</div>
								{/each}
							{/if}

							<!-- Workspaces without objects -->
							{#if workspacePreviews.length > workspacesWithObjects.length}
								<p class="text-xs text-tertiary">
									{workspacePreviews.length - workspacesWithObjects.length} workspace(s) with no objects
									to reassign
								</p>
							{/if}

							<!-- Delete user toggle -->
							{#if !reassignOnly}
								<div class="flex items-center gap-2">
									<Toggle bind:checked={deleteUser} size="xs" />
									<span class="text-sm text-secondary">Also remove user from instance</span>
								</div>
							{/if}
						</div>
					{/if}

					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						{#if workspacesWithObjects.length > 0}
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
