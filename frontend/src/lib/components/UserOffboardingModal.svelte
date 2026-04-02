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
	let hasObjects = $derived(ownedCount > 0)

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

	function downloadAffectedList() {
		if (!preview) return
		const lines: string[] = ['# Affected objects for user: ' + username, '']

		function addSection(title: string, paths: OffboardAffectedPaths | undefined) {
			if (!paths || countPaths(paths) === 0) return
			lines.push(`## ${title}`)
			for (const [kind, list] of Object.entries(paths)) {
				if (Array.isArray(list) && list.length > 0) {
					lines.push(`### ${kind}`)
					for (const p of list) lines.push(`- ${p}`)
				}
			}
			lines.push('')
		}

		addSection('Owned (will be reassigned)', preview.owned)
		addSection(
			'Executing on behalf (permissioned_as/on_behalf_of will be updated)',
			preview.executing_on_behalf
		)
		if (preview.tokens > 0) lines.push(`Tokens: ${preview.tokens} (will be deleted)`, '')
		if (preview.http_triggers > 0)
			lines.push(`HTTP triggers: ${preview.http_triggers} (webhook URLs will change)`, '')
		if (preview.email_triggers > 0)
			lines.push(`Email triggers: ${preview.email_triggers} (email addresses will change)`, '')

		const blob = new Blob([lines.join('\n')], { type: 'text/plain' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = `offboard-${username}.txt`
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
						<div class="mt-4 space-y-3">
							{#if hasObjects}
								<!-- Owned objects summary -->
								<div class="bg-surface-secondary rounded-md p-3">
									<div class="flex items-center justify-between mb-2">
										<p class="text-sm font-medium text-primary">
											{ownedCount} owned object{ownedCount !== 1 ? 's' : ''} will be reassigned
										</p>
										<Button
											variant="subtle"
											size="xs2"
											startIcon={{ icon: Download }}
											onclick={downloadAffectedList}
										>
											Export list
										</Button>
									</div>
									<div class="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs text-secondary">
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

								<!-- Reassign target -->
								<div>
									<span class="text-sm font-medium text-primary block mb-1.5">Reassign to</span>
									<div class="flex items-center gap-1 mb-2">
										<Button
											size="xs2"
											variant={targetKind === 'user' ? 'accent' : 'default'}
											onclick={() => (targetKind = 'user')}
										>
											User
										</Button>
										<Button
											size="xs2"
											variant={targetKind === 'folder' ? 'accent' : 'default'}
											onclick={() => (targetKind = 'folder')}
										>
											Folder
										</Button>
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

								<!-- Warnings -->
								{#if countPaths(preview.executing_on_behalf) > 0}
									<Alert type="info" title="Objects executing on behalf of this user">
										<p class="text-xs">
											{countPaths(preview.executing_on_behalf)} object(s) outside this user's path have
											their permissioned_as or on_behalf_of set to this user. These will be updated to
											the new operator.
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
									This user has no owned objects in this workspace.
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
