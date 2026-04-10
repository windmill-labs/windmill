<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import { AlertTriangle, CornerDownLeft, Loader2 } from 'lucide-svelte'
	import { UserService, FolderService } from '$lib/gen'
	import type { OffboardPreview } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'
	import OffboardWorkspaceSection from './OffboardWorkspaceSection.svelte'
	import { countPaths } from './offboarding-utils'

	type Props = {
		open: boolean
		username: string
		reassignOnly?: boolean
		onClose: () => void
		onComplete: () => void
	}

	let { open = $bindable(), username, reassignOnly = false, onClose, onComplete }: Props = $props()

	let preview = $state(undefined as OffboardPreview | undefined)
	let loading = $state(false)
	let submitting = $state(false)
	let conflicts: string[] = $state([])

	let doReassign = $state(true)
	let targetKind: 'user' | 'folder' = $state('user')
	let selectedUser: string | undefined = $state(undefined)
	let selectedFolder: string | undefined = $state(undefined)
	let selectedOperator: string | undefined = $state(undefined)
	let deleteUser = $state(true)

	$effect(() => {
		deleteUser = !reassignOnly
	})

	let users: Array<{ label: string; value: string }> = $state([])
	let folders: Array<{ label: string; value: string }> = $state([])

	let ownedCount = $derived(preview ? countPaths(preview.owned) : 0)
	let onBehalfCount = $derived(preview ? countPaths(preview.executing_on_behalf) : 0)
	let hasItems = $derived(ownedCount > 0 || onBehalfCount > 0)

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
		!doReassign || !hasItems || (reassignTo != null && selectedOperator != null)
	)

	$effect(() => {
		if (open) {
			doReassign = true
			targetKind = 'user'
			selectedUser = undefined
			selectedFolder = undefined
			selectedOperator = undefined
			conflicts = []
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
		submitting = true
		conflicts = []
		try {
			if (!doReassign || !hasItems) {
				await UserService.deleteUser({ workspace: $workspaceStore ?? '', username })
				sendUserToast(`User ${username} removed`)
				onComplete()
				return
			}
			if (!reassignTo) return
			const result = await UserService.offboardWorkspaceUser({
				workspace: $workspaceStore ?? '',
				username,
				requestBody: {
					reassign_to: reassignTo,
					new_on_behalf_of_user: selectedOperator,
					delete_user: deleteUser
				}
			})
			if (result.conflicts && result.conflicts.length > 0) {
				conflicts = result.conflicts
			} else {
				sendUserToast(
					deleteUser
						? `User ${username} offboarded successfully`
						: `Items reassigned from ${username} successfully`
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
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6 max-h-[80vh] overflow-y-auto"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-800/50"
						>
							<AlertTriangle class="text-red-500 dark:text-red-400" />
						</div>
						<div class="ml-4 text-left flex-1">
							<h3 class="text-lg font-medium text-primary">
								{reassignOnly ? 'Reassign user items' : 'Offboard user'}
							</h3>
							<p class="text-sm text-secondary mt-1">
								{reassignOnly
									? `Reassign items owned by ${username}`
									: `Remove ${username} and reassign their items`}
							</p>
						</div>
					</div>

					{#if loading}
						<div class="flex items-center justify-center py-8">
							<Loader2 class="animate-spin" size={24} />
							<span class="ml-2 text-sm text-secondary">Loading preview...</span>
						</div>
					{:else if preview}
						<div class="mt-4 space-y-3">
							{#if hasItems}
								{#if !reassignOnly}
									<Toggle
										bind:checked={doReassign}
										size="xs"
										options={{ right: 'Reassign items before removing' }}
									/>
								{/if}

								{#if doReassign}
									<OffboardWorkspaceSection
										{preview}
										{username}
										{deleteUser}
										bind:targetKind
										bind:selectedUser
										bind:selectedFolder
										bind:selectedOperator
										{users}
										{folders}
									/>
								{:else}
									<Alert type="warning" title="Items will not be reassigned">
										<p class="text-xs">
											All items owned by {username} ({ownedCount} owned, {onBehalfCount} running on behalf)
											will be left as-is. Triggers and runnables may stop working if the user is removed.
										</p>
									</Alert>
								{/if}
							{:else}
								<p class="text-sm text-secondary">
									This user has no items to reassign in this workspace.
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
						{#if hasItems || deleteUser}
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
