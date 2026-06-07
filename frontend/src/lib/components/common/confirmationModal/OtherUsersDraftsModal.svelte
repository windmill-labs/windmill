<script lang="ts">
	/**
	 * Banner-style modal shown on editor mount when the deployed-overlay
	 * response carries `other_drafts_users` — i.e. someone other than the
	 * authed user (or the legacy NULL-email row) also has a saved draft at
	 * this path.
	 *
	 * The list of owners is part of the get-by-path payload (so we don't
	 * fan out a second request just to populate the banner); individual
	 * drafts are fetched on-demand for the "View JSON" / "Fork" actions so
	 * the deploy-overlay response stays lean when many users are working
	 * on the same item.
	 */
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { goto } from '$lib/navigation'
	import { Users, GitFork, Braces } from 'lucide-svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'

	export type OtherDraftUser = { username?: string | null }

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		/** Workspace username of the authed user — used to namespace the
		 *  fork path (`u/{currentUserUsername}/...`). */
		currentUserUsername: string | undefined
		/** Owners list from the deployed-overlay response. Each entry has a
		 *  workspace `username` (or `null` for the legacy workspace-level
		 *  row). The authed user is already filtered out server-side. */
		otherDraftsUsers: OtherDraftUser[]
		/** Route hook: build the per-editor edit URL for a forked draft path.
		 *  Different editors live under different roots (`/scripts/edit/`,
		 *  `/flows/edit/`, ...) so the route owns the URL shape. */
		editPathFor: (forkedPath: string) => string
	}

	let { workspace, itemKind, path, currentUserUsername, otherDraftsUsers, editPathFor }: Props =
		$props()

	let isOpen = $state(otherDraftsUsers.length > 0)
	let busyFor = $state<string | null>(null)
	let jsonOpen = $state(false)
	let jsonOwnerLabel = $state('')
	let jsonValue = $state<unknown>(undefined)

	function ownerLabel(owner: OtherDraftUser): string {
		return owner.username ?? 'Legacy draft'
	}

	function ownerKey(owner: OtherDraftUser): string {
		return owner.username ?? '__legacy__'
	}

	/** Derive the fork target path. `u/{currentUser}/{leaf}_{owner}_fork`
	 *  where leaf = the last segment of the source path. For the legacy
	 *  row we use `_legacy_fork` instead of an owner username. */
	function forkPath(owner: OtherDraftUser): string {
		const leaf = path.split('/').pop() ?? path
		const ownerSuffix = owner.username ?? 'legacy'
		return `u/${currentUserUsername ?? 'me'}/${leaf}_${ownerSuffix}_fork`
	}

	async function fetchDraft(owner: OtherDraftUser): Promise<unknown> {
		return (
			await DraftService.getDraftForUser({
				workspace,
				kind: itemKind,
				path,
				username: owner.username ?? undefined
			})
		).value
	}

	async function viewJson(owner: OtherDraftUser) {
		busyFor = ownerKey(owner)
		try {
			jsonValue = await fetchDraft(owner)
			jsonOwnerLabel = ownerLabel(owner)
			jsonOpen = true
		} catch (e) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}

	async function fork(owner: OtherDraftUser) {
		busyFor = ownerKey(owner)
		try {
			const value = await fetchDraft(owner)
			const target = forkPath(owner)
			UserDraft.save(itemKind, target, value, { workspace })
			isOpen = false
			goto(editPathFor(target))
		} catch (e) {
			sendUserToast(`Could not fork draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}
</script>

<Modal2
	bind:isOpen
	title="Other users are currently working on {path}"
	fixedWidth="sm"
	fixedHeight="sm"
>
	<div class="flex flex-col w-full gap-4">
		<div class="flex gap-3 items-start">
			<Users size={20} class="text-blue-500 shrink-0 mt-0.5" />
			<p class="text-sm text-secondary">
				Their drafts are independent of yours. Open one as JSON to inspect it, or fork it into your
				own namespace to continue editing.
			</p>
		</div>

		<ul class="divide-y border-t border-b">
			{#each otherDraftsUsers as owner (ownerKey(owner))}
				<li class="flex items-center gap-3 py-2">
					<div class="flex-1 min-w-0 flex items-center gap-2">
						<span class="text-sm font-medium text-primary truncate" class:italic={!owner.username}>
							{ownerLabel(owner)}
						</span>
						{#if !owner.username}
							<Tooltip>
								Pre-migration workspace-scoped draft (no owner). Saved before drafts became per-user
								— kept around so you can recover the content, but no current user owns it.
							</Tooltip>
						{/if}
					</div>
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: Braces }}
						disabled={busyFor !== null && busyFor !== ownerKey(owner)}
						loading={busyFor === ownerKey(owner)}
						on:click={() => viewJson(owner)}
					>
						View JSON
					</Button>
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: GitFork }}
						disabled={busyFor !== null && busyFor !== ownerKey(owner)}
						loading={busyFor === ownerKey(owner)}
						on:click={() => fork(owner)}
					>
						Fork
					</Button>
				</li>
			{/each}
		</ul>

		<div class="flex justify-end">
			<Button variant="default" size="sm" on:click={() => (isOpen = false)}>Continue</Button>
		</div>
	</div>
</Modal2>

<Modal2
	bind:isOpen={jsonOpen}
	title="Draft JSON — {jsonOwnerLabel}"
	fixedWidth="lg"
	fixedHeight="lg"
>
	{#snippet headerRight()}
		<Button
			variant="default"
			size="xs"
			on:click={() => {
				navigator.clipboard?.writeText(JSON.stringify(jsonValue, null, 2))
				sendUserToast('Copied to clipboard')
			}}
		>
			Copy
		</Button>
	{/snippet}
	<div class="w-full overflow-auto">
		<pre class="text-xs whitespace-pre font-mono bg-surface-secondary rounded p-3"
			>{JSON.stringify(jsonValue ?? {}, null, 2)}</pre
		>
	</div>
</Modal2>
