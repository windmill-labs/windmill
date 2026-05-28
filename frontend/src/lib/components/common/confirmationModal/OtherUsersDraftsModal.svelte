<script lang="ts">
	/**
	 * On editor mount, queries `/drafts/users_with_draft/...`, filters out
	 * the requesting user, and surfaces every other user (and the legacy
	 * NULL-email row, if any) who has a saved draft at the path. Each
	 * entry has a "Load" button that opens the parent's `DiffDrawer` with
	 * a simple diff against the editor's current value, plus a Fork
	 * action that calls back into the parent to overwrite the local draft
	 * with the other user's content.
	 *
	 * The modal is self-fetching and self-opening — drop it into a route
	 * page with the right props and it'll only render UI when there's
	 * actually something to show.
	 */
	import { classNames } from '$lib/utils'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { Users, X } from 'lucide-svelte'
	import { DraftService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'

	type DraftOwner = { email?: string | null; created_at: string }

	type Props = {
		workspace: string
		/** UserDraft item kind — passed verbatim to the backend; matches the
		 *  `draft.typ` column. */
		itemKind: string
		path: string
		/** Current local value (post-restore-from-localstorage) shown on the
		 *  right side of the diff. */
		currentValue: unknown
		/** Email of the requesting user — filtered out of the list so the
		 *  user doesn't see themselves. */
		currentUserEmail: string | undefined
		diffDrawer: DiffDrawer | undefined
		/** Whether the user currently has unsaved local changes. Toggles the
		 *  diff drawer button label between `Fork` and
		 *  `Discard current and fork`. */
		userHasLocalDraft: boolean
		/** Called with the chosen other-user value when the user confirms
		 *  the fork. The route page wires this to its `UserDraft.save` so
		 *  the value lands in the live handle (and gets synced). */
		onFork: (otherUserValue: unknown) => void
	}

	let {
		workspace,
		itemKind,
		path,
		currentValue,
		currentUserEmail,
		diffDrawer,
		userHasLocalDraft,
		onFork
	}: Props = $props()

	let others = $state<DraftOwner[]>([])
	let open = $state(false)
	let loadingFor = $state<string | null>(null)

	onMount(async () => {
		if (!path) return
		try {
			const list = await DraftService.listUsersWithDraftOnPath({
				workspace,
				kind: itemKind,
				path
			})
			others = list.filter((u) => u.email !== currentUserEmail)
			if (others.length > 0) open = true
		} catch (e) {
			// Permission errors / 404 are expected for paths the user can't see
			// other users on. The modal just stays closed.
			console.debug('[OtherUsersDraftsModal] list failed:', e)
		}
	})

	function ownerLabel(o: DraftOwner): string {
		return o.email ?? 'Legacy workspace-level draft'
	}

	function ownerKey(o: DraftOwner): string {
		return o.email ?? '__legacy__'
	}

	async function loadDraft(owner: DraftOwner) {
		if (!diffDrawer) {
			sendUserToast('Diff drawer not ready', true)
			return
		}
		loadingFor = ownerKey(owner)
		try {
			const draft = await DraftService.getDraftForUser({
				workspace,
				kind: itemKind,
				path,
				email: owner.email ?? undefined
			})
			open = false
			diffDrawer.openDrawer()
			diffDrawer.setDiff({
				mode: 'simple',
				original: draft.value as any,
				current: currentValue as any,
				title: `${ownerLabel(owner)} <> your current`,
				button: {
					text: userHasLocalDraft ? 'Discard current and fork' : 'Fork',
					onClick: () => onFork(draft.value)
				}
			})
		} catch (e) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			loadingFor = null
		}
	}

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
</script>

{#if open}
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
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl sm:my-8 sm:w-full sm:max-w-lg sm:p-6"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-blue-100 dark:bg-blue-800/50"
						>
							<Users class="text-blue-600 dark:text-blue-300" />
						</div>
						<div class="ml-4 flex-1 text-left min-w-0">
							<h3 class="text-lg font-medium text-primary">
								Other users have drafts on <code class="break-all">{path}</code>
							</h3>
							<p class="mt-2 text-sm text-secondary">
								Click "Load" to preview their draft side-by-side with yours.
							</p>
						</div>
						<button
							type="button"
							class="text-tertiary hover:text-primary"
							aria-label="Dismiss"
							onclick={() => (open = false)}
						>
							<X size={18} />
						</button>
					</div>

					<ul class="mt-4 divide-y border-t border-b">
						{#each others as owner (ownerKey(owner))}
							<li class="flex items-center gap-3 py-2">
								<div class="flex-1 min-w-0">
									<div
										class="text-sm font-medium text-primary truncate"
										class:italic={!owner.email}
									>
										{ownerLabel(owner)}
									</div>
									<div class="text-xs text-tertiary">
										saved {new Date(owner.created_at).toLocaleString()}
									</div>
								</div>
								<Button
									variant="default"
									size="xs"
									disabled={loadingFor !== null && loadingFor !== ownerKey(owner)}
									loading={loadingFor === ownerKey(owner)}
									on:click={() => loadDraft(owner)}
								>
									Load
								</Button>
							</li>
						{/each}
					</ul>

					<div class="flex justify-end mt-4">
						<Button variant="default" size="sm" on:click={() => (open = false)}>Dismiss</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
