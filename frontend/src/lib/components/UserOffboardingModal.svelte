<script lang="ts">
	import { Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import { AlertTriangle, CornerDownLeft, Loader2 } from 'lucide-svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { UserService, FolderService } from '$lib/gen'
	import type { OffboardPreview } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'

	type Props = {
		open: boolean
		username: string
		/** If true, the "delete user" checkbox defaults to unchecked */
		reassignOnly?: boolean
		onClose: () => void
		onComplete: () => void
	}

	let { open = $bindable(), username, reassignOnly = false, onClose, onComplete }: Props = $props()

	let preview: OffboardPreview | undefined = $state(undefined)
	let loading = $state(false)
	let submitting = $state(false)
	let conflicts: string[] = $state([])

	// Form state
	let targetKind: 'user' | 'folder' = $state('user')
	let selectedUser: string | undefined = $state(undefined)
	let selectedFolder: string | undefined = $state(undefined)
	let selectedOperator: string | undefined = $state(undefined)
	let deleteUser = $state(true)

	$effect(() => {
		deleteUser = !reassignOnly
	})

	// Data for selectors
	let users: Array<{ label: string; value: string }> = $state([])
	let folders: Array<{ label: string; value: string }> = $state([])

	function previewHasObjects(p: OffboardPreview | undefined): boolean {
		if (!p) return false
		return (
			p.scripts > 0 ||
			p.flows > 0 ||
			p.apps > 0 ||
			p.resources > 0 ||
			p.variables > 0 ||
			p.schedules > 0 ||
			p.triggers > 0 ||
			p.tokens > 0
		)
	}

	let hasObjects = $derived(previewHasObjects(preview))

	let reassignTo = $derived(
		targetKind === 'user'
			? selectedUser
				? `u/${selectedUser}`
				: undefined
			: selectedFolder
				? `f/${selectedFolder}`
				: undefined
	)

	let canSubmit = $derived(
		reassignTo != null && (targetKind === 'user' || selectedOperator != null)
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
			const workspace = $workspaceStore ?? ''
			const [previewResult, usernamesList, foldersList] = await Promise.all([
				UserService.offboardPreview({ workspace, username }),
				UserService.listUsernames({ workspace }),
				FolderService.listFolders({ workspace })
			])
			preview = previewResult
			users = usernamesList.filter((u) => u !== username).map((u) => ({ label: u, value: u }))
			folders = foldersList.map((f) => ({ label: f.name, value: f.name }))
		} catch (e) {
			sendUserToast('Failed to load offboard preview', true)
			onClose()
		} finally {
			loading = false
		}
	}

	async function submit() {
		if (!reassignTo) return
		submitting = true
		conflicts = []
		try {
			const result = await UserService.offboardWorkspaceUser({
				workspace: $workspaceStore ?? '',
				username,
				requestBody: {
					reassign_to: reassignTo,
					new_operator: targetKind === 'folder' ? selectedOperator : undefined,
					delete_user: deleteUser
				}
			})
			if (result.conflicts && result.conflicts.length > 0) {
				conflicts = result.conflicts
			} else {
				sendUserToast(
					deleteUser
						? `User ${username} offboarded successfully`
						: `Objects reassigned from ${username} successfully`
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
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-800/50"
						>
							<AlertTriangle class="text-red-500 dark:text-red-400" />
						</div>
						<div class="ml-4 text-left flex-1">
							<h3 class="text-lg font-medium text-primary">
								{reassignOnly ? 'Reassign user objects' : 'Offboard user'}
							</h3>
							<p class="text-sm text-secondary mt-1">
								{reassignOnly
									? `Reassign objects owned by ${username}`
									: `Remove ${username} and reassign their objects`}
							</p>
						</div>
					</div>

					{#if loading}
						<div class="flex items-center justify-center py-8">
							<Loader2 class="animate-spin" size={24} />
							<span class="ml-2 text-sm text-secondary">Loading preview...</span>
						</div>
					{:else if preview}
						<div class="mt-4 space-y-4">
							<!-- Object counts -->
							{#if hasObjects}
								<div class="bg-surface-secondary rounded-md p-3">
									<p class="text-sm font-medium text-primary mb-2">Owned objects:</p>
									<div class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm text-secondary">
										{#if preview.scripts > 0}<span>Scripts: {preview.scripts}</span>{/if}
										{#if preview.flows > 0}<span>Flows: {preview.flows}</span>{/if}
										{#if preview.apps > 0}<span>Apps: {preview.apps}</span>{/if}
										{#if preview.resources > 0}<span>Resources: {preview.resources}</span>{/if}
										{#if preview.variables > 0}<span>Variables: {preview.variables}</span>{/if}
										{#if preview.schedules > 0}<span>Schedules: {preview.schedules}</span>{/if}
										{#if preview.triggers > 0}<span>Triggers: {preview.triggers}</span>{/if}
										{#if preview.tokens > 0}<span>Tokens: {preview.tokens}</span>{/if}
									</div>
								</div>

								<!-- Reassign target -->
								<div>
									<label class="text-sm font-medium text-primary block mb-1"
										>Reassign objects to</label
									>
									<div class="flex items-center gap-2 mb-2">
										<button
											class={classNames(
												'px-3 py-1 text-sm rounded-md border',
												targetKind === 'user'
													? 'bg-surface-selected border-border-selected text-primary'
													: 'border-border bg-surface text-secondary'
											)}
											onclick={() => (targetKind = 'user')}
										>
											User
										</button>
										<button
											class={classNames(
												'px-3 py-1 text-sm rounded-md border',
												targetKind === 'folder'
													? 'bg-surface-selected border-border-selected text-primary'
													: 'border-border bg-surface text-secondary'
											)}
											onclick={() => (targetKind = 'folder')}
										>
											Folder
										</button>
									</div>
									{#if targetKind === 'user'}
										<Select
											items={users}
											bind:value={selectedUser}
											placeholder="Select a user..."
										/>
									{:else}
										<Select
											items={folders}
											bind:value={selectedFolder}
											placeholder="Select a folder..."
										/>
										<div class="mt-2">
											<label class="text-xs text-secondary block mb-1"
												>Run schedules/triggers as</label
											>
											<Select
												items={users}
												bind:value={selectedOperator}
												placeholder="Select operator user..."
											/>
										</div>
									{/if}
								</div>

								<!-- Warnings for operator references outside user's path -->
								{#if (preview.scripts_as_operator ?? 0) > 0 || (preview.flows_as_operator ?? 0) > 0 || (preview.apps_as_operator ?? 0) > 0 || (preview.schedules_as_operator ?? 0) > 0 || (preview.triggers_as_operator ?? 0) > 0 || (preview.tokens ?? 0) > 0}
									<div class="bg-yellow-50 dark:bg-yellow-900/20 rounded-md p-3">
										<p class="text-sm font-medium text-yellow-700 dark:text-yellow-400 mb-1">
											Additional references (will also be updated):
										</p>
										<div class="text-xs text-yellow-600 dark:text-yellow-300 space-y-0.5">
											{#if (preview.scripts_as_operator ?? 0) > 0}
												<p
													>{preview.scripts_as_operator} script(s) with on_behalf_of set to this user</p
												>
											{/if}
											{#if (preview.flows_as_operator ?? 0) > 0}
												<p>{preview.flows_as_operator} flow(s) with on_behalf_of set to this user</p
												>
											{/if}
											{#if (preview.apps_as_operator ?? 0) > 0}
												<p>{preview.apps_as_operator} app(s) with on_behalf_of set to this user</p>
											{/if}
											{#if (preview.schedules_as_operator ?? 0) > 0}
												<p
													>{preview.schedules_as_operator} schedule(s) running as this user (not under
													their path)</p
												>
											{/if}
											{#if (preview.triggers_as_operator ?? 0) > 0}
												<p
													>{preview.triggers_as_operator} trigger(s) running as this user (not under
													their path)</p
												>
											{/if}
											{#if (preview.tokens ?? 0) > 0}
												<p
													>{preview.tokens} token(s) owned by this user (not handled, webhooks may break)</p
												>
											{/if}
										</div>
									</div>
								{/if}

								<!-- Delete user toggle -->
								{#if !reassignOnly}
									<div class="flex items-center gap-2">
										<Toggle bind:checked={deleteUser} size="xs" />
										<span class="text-sm text-secondary">Also remove user from workspace</span>
									</div>
								{/if}
							{:else}
								<p class="text-sm text-secondary">
									This user has no owned objects or tokens in this workspace.
								</p>
							{/if}

							<!-- Conflicts -->
							{#if conflicts.length > 0}
								<div class="bg-red-50 dark:bg-red-900/30 rounded-md p-3">
									<p class="text-sm font-medium text-red-700 dark:text-red-400 mb-1">
										Path conflicts detected - resolve before retrying:
									</p>
									<ul
										class="text-xs text-red-600 dark:text-red-300 list-disc list-inside max-h-32 overflow-y-auto"
									>
										{#each conflicts as conflict}
											<li>{conflict}</li>
										{/each}
									</ul>
								</div>
							{/if}
						</div>
					{/if}

					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						{#if hasObjects}
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
							<Button
								onclick={() => {
									// No objects — just delete directly if needed
									onClose()
								}}
								variant="accent"
								size="sm"
							>
								Close
							</Button>
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
