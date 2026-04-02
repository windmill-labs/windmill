<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { classNames } from '$lib/utils'
	import { AlertTriangle, CornerDownLeft, Download, Loader2 } from 'lucide-svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { UserService, FolderService } from '$lib/gen'
	import type { OffboardAffectedPaths } from '$lib/gen'
	import type { OffboardPreview } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'

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

	// Auto-default selectedOperator to target user when switching to user target
	$effect(() => {
		if (targetKind === 'user' && selectedUser && !selectedOperator) {
			selectedOperator = selectedUser
		}
	})

	let canSubmit = $derived((ownedCount === 0 || reassignTo != null) && selectedOperator != null)

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

	function downloadAffectedCsv() {
		if (!preview) return
		const rows: string[][] = [['category', 'type', 'path']]

		function addRows(category: string, paths: OffboardAffectedPaths | undefined) {
			if (!paths) return
			for (const [kind, list] of Object.entries(paths)) {
				if (Array.isArray(list)) {
					for (const p of list) rows.push([category, kind, p])
				}
			}
		}

		addRows('owned', preview.owned)
		addRows('executing_on_behalf', preview.executing_on_behalf)
		addRows('referencing', preview.referencing)

		const csv = rows.map((r) => r.map((c) => `"${c.replace(/"/g, '""')}"`).join(',')).join('\n')
		const blob = new Blob([csv], { type: 'text/csv' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = `offboard-${username}.csv`
		a.click()
		URL.revokeObjectURL(url)
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
								<!-- Summary boxes side by side -->
								<div class="grid grid-cols-2 gap-2">
									{#if ownedCount > 0}
										<div class="bg-surface-secondary rounded-md p-3">
											<p class="text-xs font-medium text-primary mb-1">Owned items ({ownedCount})</p
											>
											<p class="text-xs text-tertiary mb-1.5">Under u/{username}/, will be moved.</p
											>
											<div class="flex flex-col gap-0.5 text-xs text-secondary">
												{#if (preview.owned.scripts?.length ?? 0) > 0}<span
														>{preview.owned.scripts?.length} scripts</span
													>{/if}
												{#if (preview.owned.flows?.length ?? 0) > 0}<span
														>{preview.owned.flows?.length} flows</span
													>{/if}
												{#if (preview.owned.apps?.length ?? 0) > 0}<span
														>{preview.owned.apps?.length} apps</span
													>{/if}
												{#if (preview.owned.resources?.length ?? 0) > 0}<span
														>{preview.owned.resources?.length} resources</span
													>{/if}
												{#if (preview.owned.variables?.length ?? 0) > 0}<span
														>{preview.owned.variables?.length} variables</span
													>{/if}
												{#if (preview.owned.schedules?.length ?? 0) > 0}<span
														>{preview.owned.schedules?.length} schedules</span
													>{/if}
												{#if (preview.owned.triggers?.length ?? 0) > 0}<span
														>{preview.owned.triggers?.length} triggers</span
													>{/if}
												{#if preview.tokens > 0}<span>{preview.tokens} tokens (deleted)</span>{/if}
											</div>
										</div>
									{/if}
									{#if onBehalfCount > 0}
										<div class="bg-surface-secondary rounded-md p-3">
											<p class="text-xs font-medium text-primary mb-1"
												>Running on behalf ({onBehalfCount})</p
											>
											<p class="text-xs text-tertiary mb-1.5"
												>permissioned_as / on_behalf_of will be updated.</p
											>
											<div class="flex flex-col gap-0.5 text-xs text-secondary">
												{#if (preview.executing_on_behalf.scripts?.length ?? 0) > 0}<span
														>{preview.executing_on_behalf.scripts?.length} scripts</span
													>{/if}
												{#if (preview.executing_on_behalf.flows?.length ?? 0) > 0}<span
														>{preview.executing_on_behalf.flows?.length} flows</span
													>{/if}
												{#if (preview.executing_on_behalf.apps?.length ?? 0) > 0}<span
														>{preview.executing_on_behalf.apps?.length} apps</span
													>{/if}
												{#if (preview.executing_on_behalf.schedules?.length ?? 0) > 0}<span
														>{preview.executing_on_behalf.schedules?.length} schedules</span
													>{/if}
												{#if (preview.executing_on_behalf.triggers?.length ?? 0) > 0}<span
														>{preview.executing_on_behalf.triggers?.length} triggers</span
													>{/if}
											</div>
										</div>
									{/if}
								</div>

								<div class="flex justify-end">
									<Button
										variant="subtle"
										size="xs2"
										startIcon={{ icon: Download }}
										onclick={downloadAffectedCsv}
									>
										Export CSV
									</Button>
								</div>

								<!-- Reassign target for owned items -->
								{#if ownedCount > 0}
									<div>
										<span class="text-sm font-medium text-primary block mb-1.5"
											>Reassign owned items to</span
										>
										<div class="flex items-center gap-1 mb-2">
											<Button
												size="xs2"
												variant={targetKind === 'user' ? 'accent' : 'default'}
												onclick={() => (targetKind = 'user')}>User</Button
											>
											<Button
												size="xs2"
												variant={targetKind === 'folder' ? 'accent' : 'default'}
												onclick={() => (targetKind = 'folder')}>Folder</Button
											>
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
										{/if}
									</div>
								{/if}

								<!-- New on_behalf_of user (always shown) -->
								<div>
									<span class="text-sm font-medium text-primary block mb-1.5"
										>New on_behalf_of user</span
									>
									<p class="text-xs text-tertiary mb-1.5"
										>User identity for permissioned_as on schedules/triggers and on_behalf_of on
										scripts/flows/apps.</p
									>
									<Select
										items={users}
										bind:value={selectedOperator}
										placeholder="Select a user..."
									/>
								</div>

								<!-- Warnings -->
								{#if countPaths(preview.referencing) > 0}
									<Alert type="warning" title="Content referencing this user's paths">
										<p class="text-xs">
											{countPaths(preview.referencing)} item(s) contain references to this user's paths
											in their content or values. These references will break after reassignment. Use
											the export list for details.
										</p>
									</Alert>
								{/if}

								{#if preview.http_triggers > 0 || preview.email_triggers > 0}
									<Alert type="warning" title="Webhook and email trigger URLs will change">
										<p class="text-xs">
											{#if preview.http_triggers > 0}{preview.http_triggers} HTTP trigger(s) will have
												new webhook URLs.{/if}
											{#if preview.email_triggers > 0}{preview.email_triggers} email trigger(s) will
												have new addresses.{/if}
											Update any external integrations that reference these endpoints.
										</p>
									</Alert>
								{/if}

								<!-- Delete user toggle -->
								{#if !reassignOnly}
									<div class="flex items-center gap-2 pt-1">
										<Toggle bind:checked={deleteUser} size="xs" />
										<span class="text-sm text-secondary">Also remove user from workspace</span>
									</div>
								{/if}
							{:else}
								<p class="text-sm text-secondary">
									This user has no items to reassign in this workspace.
								</p>
							{/if}

							<!-- Conflicts -->
							{#if conflicts.length > 0}
								<Alert type="error" title="Path conflicts detected">
									<p class="text-xs mb-1">Resolve these before retrying:</p>
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
						{#if hasItems}
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
