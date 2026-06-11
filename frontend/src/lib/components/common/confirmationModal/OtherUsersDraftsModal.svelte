<script lang="ts">
	/**
	 * Modal opened on demand (from the AutosaveIndicator popover or the
	 * home-page DraftBadge popover) when other workspace users have a
	 * draft at the same path. The owner list is part of the
	 * deployed-overlay / list payload; individual drafts are fetched
	 * on-demand for the "View JSON" / "Fork" actions so the response
	 * stays lean when many users are working on the same item.
	 *
	 * The parent controls visibility — bind to `isOpen`. The modal does
	 * NOT auto-open on mount; that used to surprise users every time
	 * they opened an item with collaborators.
	 */
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Users, GitFork, Braces } from 'lucide-svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { forkDraftToImport } from '$lib/components/forkDraftToImport'

	export type OtherDraftUser = { username?: string | null }

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		/** Owners list from the deployed-overlay response. Each entry has a
		 *  workspace `username` (or `null` for the legacy workspace-level
		 *  row). The authed user is already filtered out server-side. */
		otherDraftsUsers: OtherDraftUser[]
		/** Controlled visibility — bind from the parent. */
		isOpen: boolean
	}

	let { workspace, itemKind, path, otherDraftsUsers, isOpen = $bindable() }: Props = $props()
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
			// Close the banner BEFORE the navigation so the user sees the
			// modal disappear on click. Without this the modal stays
			// visible during the navigation tear-down — Svelte hasn't
			// torn down the previous route's components by the time
			// `goto` returns, so the banner lingers on top of the
			// destination editor for a beat.
			isOpen = false
			// Import-style handoff: seed a brand-new own item from the
			// fetched value (no immediate server save, fresh owned path).
			forkDraftToImport(itemKind, value)
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
	closeOnOutsideClick={!jsonOpen}
>
	<div class="flex flex-col w-full gap-4">
		<div class="flex gap-3 items-start">
			<Users size={20} class="text-blue-500 shrink-0 mt-0.5" />
			<p class="text-sm text-secondary">
				Their drafts are independent of yours. For advanced collaboration, consider using <a
					target="_blank"
					href="https://www.windmill.dev/docs/advanced/workspace_forks">workspace forks (EE)</a
				>
			</p>
		</div>

		<ul class="divide-y border-t border-b flex-1 overflow-y-auto">
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
			<Button variant="default" size="sm" on:click={() => (isOpen = false)}>Close</Button>
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
