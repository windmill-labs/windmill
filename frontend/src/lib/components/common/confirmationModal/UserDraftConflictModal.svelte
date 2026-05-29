<script lang="ts">
	/**
	 * Surfaces UserDraft sync conflicts one at a time. Mounted once near the
	 * top of the tree (root layout) so any UserDraft.save call can enqueue
	 * a conflict via `UserDraftConflictStore.enqueue` and have it shown to
	 * the user without coordinating with a route component.
	 *
	 * Two conflict shapes:
	 *   • upsert rejected: `incoming_value` is non-null. The user tried to
	 *     save and lost the race. Offer "Overwrite server draft" / "Load
	 *     server draft".
	 *   • delete rejected: `incoming_value` is null. The user tried to
	 *     discard and lost the race. Offer "Delete anyway" / "Load server
	 *     draft".
	 */
	import { classNames } from '$lib/utils'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { AlertTriangle, CornerDownLeft } from 'lucide-svelte'
	import { UserDraftConflictStore } from '$lib/userDraftConflictStore.svelte'
	import { UserDraftDbSyncer, syncDrafts } from '$lib/userDraftDbSyncer.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { sendUserToast } from '$lib/toast'

	const conflict = $derived(UserDraftConflictStore.current)
	const open = $derived(conflict !== undefined)
	const isDeleteAttempt = $derived(
		conflict !== undefined &&
			(conflict.rejected.incoming_value === null || conflict.rejected.incoming_value === undefined)
	)

	const lastSyncDate = $derived(UserDraftDbSyncer.getLastSync())

	let busy = $state(false)

	async function forceLocal() {
		if (!conflict) return
		busy = true
		try {
			await syncDrafts({
				workspace: conflict.workspace,
				drafts: [
					{
						itemKind: conflict.itemKind,
						path: conflict.rejected.path,
						value: isDeleteAttempt ? null : conflict.rejected.incoming_value,
						force: true
					}
				]
			})
			UserDraftConflictStore.dismiss()
		} catch (e) {
			sendUserToast(`Could not apply local change: ${e.body ?? e.message}`, true)
		} finally {
			busy = false
		}
	}

	function loadServer() {
		if (!conflict) return
		UserDraft.save(conflict.itemKind, conflict.rejected.path, conflict.rejected.server_value, {
			workspace: conflict.workspace
		})
		UserDraftConflictStore.dismiss()
	}

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
</script>

{#if open && conflict}
	<div transition:fadeFast|local class="fixed top-0 bottom-0 left-0 right-0 z-[9999]" role="dialog">
		<div
			class={classNames(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				'ease-out duration-300 opacity-100'
			)}
		></div>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6 ease-out duration-300 opacity-100 translate-y-0 sm:scale-100"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-amber-100 dark:bg-amber-800/50"
						>
							<AlertTriangle class="text-amber-600 dark:text-amber-300" />
						</div>
						<div class="ml-4 flex-1 text-left">
							<h3 class="text-lg font-medium text-primary">
								{#if isDeleteAttempt}
									Couldn't discard your draft for <code>{conflict.rejected.path}</code>
								{:else}
									Your draft for <code>{conflict.rejected.path}</code> is out of date
								{/if}
							</h3>
							<p class="mt-2 text-sm text-secondary">
								{#if isDeleteAttempt}
									Another session saved a newer draft for this {conflict.itemKind} since this tab last
									synced — discarding now would erase changes you haven't seen.
								{:else}
									Another session saved a newer draft for this {conflict.itemKind} since this tab last
									synced.
								{/if}
							</p>
							<dl
								class="mt-3 text-xs text-tertiary grid grid-cols-[max-content_1fr] gap-x-2 gap-y-1"
							>
								<dt>Last sync from this tab:</dt>
								<dd>
									{lastSyncDate ? new Date(lastSyncDate).toLocaleString() : 'never'}
								</dd>
								<dt>Server draft saved at:</dt>
								<dd>{new Date(conflict.rejected.server_created_at).toLocaleString()}</dd>
							</dl>
						</div>
					</div>
					<div class="flex items-center justify-end gap-2 mt-4 flex-wrap">
						<Button
							disabled={busy}
							on:click={loadServer}
							variant="default"
							size="sm"
							shortCut={{ key: 'Esc', withoutModifier: true }}
						>
							Load server draft
						</Button>
						<Button
							disabled={busy}
							loading={busy}
							on:click={forceLocal}
							color="dark"
							size="sm"
							shortCut={{ Icon: CornerDownLeft, withoutModifier: true }}
							variant="accent"
						>
							{isDeleteAttempt ? 'Discard anyway' : 'Overwrite server draft'}
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
