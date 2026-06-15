<script lang="ts">
	/**
	 * Surfaces the conflict snapshot left by `UserDraftDbSyncer.postSave`
	 * when the server rejects a save because the row's `created_at` has
	 * advanced past our `last_sync` (another tab/browser/user pushed an
	 * intervening write). The route mounts one of these per editor: it
	 * reads the reactive conflict handle and offers two resolutions —
	 * pull the remote (discards local edits) or push over it.
	 */
	import { UserDraftDbSyncer, type UserDraftLastSyncQuery } from '$lib/userDraftDbSyncer.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertTriangle } from 'lucide-svelte'

	type Props = {
		query: UserDraftLastSyncQuery
		/** Editor-side reload — re-fetches the deployed-overlay response,
		 *  resets in-memory state, and (implicitly via the loader calling
		 *  `recordRemoteSync`) updates the local `last_sync` to the
		 *  server's clock. The modal awaits this before closing. */
		onLoadFromServer: () => Promise<void> | void
		/** Current local draft value to overwrite the server with. The
		 *  modal passes it through `UserDraftDbSyncer.overwrite`. */
		getLocalDraft: () => unknown
	}

	let { query, onLoadFromServer, getLocalDraft }: Props = $props()

	const conflictHandle = $derived(UserDraftDbSyncer.getConflict(query))
	let isOpen = $derived(conflictHandle.conflict !== undefined)
	let busy = $state(false)

	async function loadFromServer() {
		busy = true
		try {
			await onLoadFromServer()
			UserDraftDbSyncer.clearConflict(query)
		} finally {
			busy = false
		}
	}

	async function overwriteServer() {
		busy = true
		try {
			await UserDraftDbSyncer.overwrite({
				workspace: query.workspace,
				itemKind: query.itemKind,
				path: query.path,
				value: getLocalDraft()
			})
		} finally {
			busy = false
		}
	}
</script>

<Modal2 bind:isOpen title="Draft out of sync" fixedWidth="sm" fixedHeight="adaptive">
	<div class="flex flex-col w-full gap-4">
		<div class="flex gap-3 items-start flex-1">
			<AlertTriangle size={20} class="text-yellow-500 shrink-0 mt-0.5" />
			<div class="text-sm text-primary flex flex-col gap-1">
				<p>
					Another tab, browser, or AI agent saved a newer version of this draft. Your autosave was
					rejected to avoid overwriting their work.
				</p>
				{#if conflictHandle.conflict}
					<p class="text-xs text-secondary">
						Server timestamp: {new Date(conflictHandle.conflict.serverTimestamp).toLocaleString()}
					</p>
				{/if}
			</div>
		</div>

		<div class="flex justify-end gap-2">
			<Button
				variant="default"
				size="sm"
				disabled={busy}
				on:click={() => UserDraftDbSyncer.clearConflict(query)}
			>
				Dismiss
			</Button>
			<Button variant="default" size="sm" disabled={busy} on:click={overwriteServer}>
				Overwrite the remote
			</Button>
			<Button variant="accent" size="sm" loading={busy} on:click={loadFromServer}>
				Load from server
			</Button>
		</div>
	</div>
</Modal2>
