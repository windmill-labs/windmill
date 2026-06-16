<script lang="ts">
	/**
	 * On-demand modal (from the AutosaveIndicator or DraftBadge popover) listing
	 * other users' drafts at this path. The owner list rides the overlay/list
	 * payload; individual drafts are fetched lazily for View Diff / Load.
	 * Parent-controlled via `isOpen` — never auto-opens.
	 */
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Users, GitFork, GitCompareArrows } from 'lucide-svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { fetchDeployedValueForDiff } from '$lib/components/otherUserDraftDiff'
	import { forkDraftToImport } from '$lib/components/forkDraftToImport'

	export type OtherDraftUser = { username?: string | null }

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		/** Owners from the overlay response (`username`, or `null` for the
		 *  legacy row). The authed user is filtered out server-side. */
		otherDraftsUsers: OtherDraftUser[]
		/** No deployed row exists (never deployed): hides View Diff, since there's
		 *  no deployed baseline to diff against. */
		draftOnly?: boolean
		/** Controlled visibility — bind from the parent. */
		isOpen: boolean
	}

	let {
		workspace,
		itemKind,
		path,
		otherDraftsUsers,
		draftOnly = false,
		isOpen = $bindable()
	}: Props = $props()
	let busyFor = $state<string | null>(null)
	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let diffOpen = $state(false)

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

	async function viewDiff(owner: OtherDraftUser) {
		busyFor = ownerKey(owner)
		try {
			const [draftValue, deployed] = await Promise.all([
				fetchDraft(owner),
				fetchDeployedValueForDiff(workspace, itemKind, path)
			])
			diffOpen = true
			diffDrawer?.openDrawer()
			diffDrawer?.setDiff({
				mode: 'simple',
				title: `${ownerLabel(owner)}'s draft vs deployed`,
				original: deployed,
				current: draftValue as any
			})
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
			// Close before navigating — `goto` returns before Svelte tears down
			// the route, so the modal would otherwise linger on the destination.
			isOpen = false
			// Seed a brand-new own item from the fetched value (no server save).
			forkDraftToImport(itemKind, value, path)
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
	closeOnOutsideClick={!diffOpen}
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
					{#if !draftOnly}
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: GitCompareArrows }}
							disabled={busyFor !== null && busyFor !== ownerKey(owner)}
							loading={busyFor === ownerKey(owner)}
							on:click={() => viewDiff(owner)}
						>
							View Diff
						</Button>
					{/if}
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

<DiffDrawer
	bind:this={diffDrawer}
	isFlow={itemKind === 'flow'}
	on:close={() => (diffOpen = false)}
/>
