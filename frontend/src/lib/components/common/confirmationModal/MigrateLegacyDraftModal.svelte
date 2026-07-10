<script lang="ts">
	/**
	 * Admin-only resolution of a LEGACY (workspace-level, no-owner) draft: delete
	 * it, or assign it to yourself as a normal per-user draft. Opened from the
	 * home-page draft popover and the in-editor "other users' drafts" modal.
	 */
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Trash2, UserCheck, Wrench, TriangleAlert } from 'lucide-svelte'

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		isOpen: boolean
		/** You already have your own draft at this path: "Assign to self" would
		 *  replace it, so confirm first. */
		ownDraftExists?: boolean
		/** Refresh the caller's view once the legacy row is gone/reassigned. */
		onMigrated?: () => void | Promise<void>
	}

	let {
		workspace,
		itemKind,
		path,
		isOpen = $bindable(),
		ownDraftExists = false,
		onMigrated
	}: Props = $props()

	let busy = $state<'delete' | 'assign_to_self' | null>(null)
	// "Assign to self" would overwrite our own draft — show the confirm step first.
	let confirmingAssign = $state(false)

	// Reset the confirm step whenever the modal (re)opens.
	$effect(() => {
		if (isOpen) confirmingAssign = false
	})

	function onAssignClick() {
		if (ownDraftExists) confirmingAssign = true
		else void run('assign_to_self')
	}

	async function run(action: 'delete' | 'assign_to_self') {
		busy = action
		try {
			await DraftService.migrateLegacyDraft({
				workspace,
				kind: itemKind,
				path,
				requestBody: { action }
			})
			sendUserToast(action === 'delete' ? 'Legacy draft deleted' : 'Legacy draft assigned to you')
			isOpen = false
			await onMigrated?.()
		} catch (e: any) {
			sendUserToast(`Could not migrate legacy draft: ${e.body ?? e.message}`, true)
		} finally {
			busy = null
		}
	}
</script>

<Modal2 bind:isOpen title="Migrate legacy draft — {path}" fixedWidth="sm" fixedHeight="adaptive">
	<div class="flex flex-col w-full gap-4">
		{#if confirmingAssign}
			<div class="flex gap-3 items-start">
				<TriangleAlert size={20} class="text-yellow-500 shrink-0 mt-0.5" />
				<p class="text-sm text-secondary">
					You already have your own draft at <span class="font-medium text-primary">{path}</span>.
					Assigning this legacy draft to yourself will
					<span class="font-semibold">replace your current draft</span>. This can't be undone.
				</p>
			</div>
			<div class="flex gap-2 ml-auto">
				<Button variant="default" size="sm" on:click={() => (confirmingAssign = false)}>
					Cancel
				</Button>
				<Button
					variant="default"
					destructive
					size="sm"
					startIcon={{ icon: UserCheck }}
					loading={busy === 'assign_to_self'}
					on:click={() => run('assign_to_self')}
				>
					Replace my draft
				</Button>
			</div>
		{:else}
			<div class="flex gap-3 items-start">
				<Wrench size={20} class="text-blue-500 shrink-0 mt-0.5" />
				<p class="text-sm text-secondary">
					This is a pre-migration workspace-level draft with no owner. As an admin you can delete
					it, or assign it to yourself to keep editing it as your own draft.
				</p>
			</div>
			<div class="flex gap-2 ml-auto">
				<Button variant="default" size="sm" on:click={() => (isOpen = false)}>Cancel</Button>
				<Button
					variant="default"
					destructive
					size="sm"
					startIcon={{ icon: Trash2 }}
					loading={busy === 'delete'}
					disabled={busy !== null && busy !== 'delete'}
					on:click={() => run('delete')}
				>
					Delete draft
				</Button>
				<Button
					variant="default"
					size="sm"
					startIcon={{ icon: UserCheck }}
					loading={busy === 'assign_to_self'}
					disabled={busy !== null && busy !== 'assign_to_self'}
					on:click={onAssignClick}
				>
					Assign to self
				</Button>
			</div>
		{/if}
	</div>
</Modal2>
