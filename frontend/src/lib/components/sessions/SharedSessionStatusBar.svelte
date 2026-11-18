<script lang="ts" module>
	import { lazyModule } from '$lib/lazyModule.svelte'

	const diffDrawerModule = lazyModule(() => import('$lib/components/DiffDrawer.svelte'))
</script>

<script lang="ts">
	import { tick } from 'svelte'
	import { ExternalLink, GitCompareArrows, Lock } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import ArtifactsSegment from '$lib/components/copilot/chat/artifacts/ArtifactsSegment.svelte'
	import type { PersistedArtifact } from '$lib/components/copilot/chat/artifacts/artifactsDB'
	import { itemHref } from '$lib/components/copilot/chat/workspaceItems.svelte'
	import type { WindmillItemKind } from '$lib/components/copilot/chat/workspaceItems.svelte'
	import { fetchDeployedValueForDiff } from '$lib/components/otherUserDraftDiff'
	import { DraftService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import SessionStatusPopover from './SessionStatusPopover.svelte'
	import { editsToken } from './sessionDeployModel'
	import type { SharedSessionEdit } from './sessionSharing'
	import { TOKEN_TRIGGER_CLASS } from './SessionStatusToken.svelte'

	interface Props {
		edits: SharedSessionEdit[]
		editsWorkspace: string | undefined
		/** The author's username. */
		author: string
		/** The reader may ask for the author's drafts in `editsWorkspace`. */
		readsDrafts: boolean
		/** Why the reader cannot, shown under the list once it is known. */
		draftsLockedReason: string | undefined
		/** Where the reader signs in to lift the lock, when they have no session. */
		draftsSignInHref?: string
		artifacts: PersistedArtifact[]
		openArtifact: (artifact: PersistedArtifact) => void
	}

	let {
		edits,
		editsWorkspace,
		author,
		readsDrafts,
		draftsLockedReason,
		draftsSignInHref,
		artifacts,
		openArtifact
	}: Props = $props()

	const token = $derived(
		editsToken({
			draft: edits.filter((e) => e.draft).length,
			deployed: edits.filter((e) => !e.draft).length
		})
	)

	function deployedHref(edit: SharedSessionEdit): string | undefined {
		// Without it, `itemHref` would link the reader's own workspace's item at that path.
		if (edit.draft || !editsWorkspace) return undefined
		const kind = (edit.kind === 'raw_app' ? 'app' : edit.kind) as WindmillItemKind
		const href = itemHref(
			{ kind, path: edit.path, rawApp: edit.kind === 'raw_app' },
			editsWorkspace
		)
		return href.startsWith('#') ? undefined : href
	}

	// Only these kinds share drafts across users (the server's `shares_drafts_across_users`);
	// the others stay private to their author. A never-deployed draft is stored under the
	// author's own path, which the server lets only them read.
	const SHARED_DRAFT_KINDS = new Set(['script', 'flow', 'app', 'raw_app'])
	function opensDraft(edit: SharedSessionEdit): boolean {
		return (
			edit.draft && readsDrafts && !!editsWorkspace && !!edit.draftPath && !draftPrivateReason(edit)
		)
	}

	/** Why no reader but the author can open this draft, whoever the reader is. */
	function draftPrivateReason(edit: SharedSessionEdit): string | undefined {
		if (edit.draftOnly) return `Never deployed, so only ${author} can open this draft`
		if (!edit.draftKind || !SHARED_DRAFT_KINDS.has(edit.draftKind)) {
			return `Drafts of a ${edit.kind.replaceAll('_', ' ')} stay private to their author`
		}
		return undefined
	}

	const hasLockedRow = $derived(edits.some((e) => (e.draft ? !opensDraft(e) : !deployedHref(e))))

	function rowTitle(edit: SharedSessionEdit): string {
		if (!edit.draft) {
			if (deployedHref(edit)) return `Open ${edit.path}`
			return draftsLockedReason ? `${edit.path}: ${draftsLockedReason}` : edit.path
		}
		if (opensDraft(edit)) return `View ${author}'s draft of ${edit.path}`
		return `${edit.path}: ${draftPrivateReason(edit) ?? draftsLockedReason ?? `only ${author} can open this draft`}`
	}

	let editsOpen = $state(false)
	let diffDrawer: DiffDrawer | undefined = $state(undefined)

	// The editor's own draft loading navigates, and leaving this page reloads it, which would
	// drop the loaded draft: the diff is read here instead, as the home page's draft badge does.
	async function viewDraft(edit: SharedSessionEdit) {
		if (!opensDraft(edit)) return
		const workspace = editsWorkspace!
		const kind = edit.draftKind!
		try {
			const [draft, deployed] = await Promise.all([
				DraftService.getDraftForUser({
					workspace,
					kind,
					path: edit.draftPath!,
					username: author
				}),
				fetchDeployedValueForDiff(workspace, kind, edit.path),
				diffDrawerModule.load()
			])
			await tick()
			diffDrawer?.openDrawer()
			diffDrawer?.setDiff({
				mode: 'simple',
				title: `${author}'s draft vs deployed`,
				original: deployed ?? {},
				current: draft.value as any
			})
		} catch (e: any) {
			sendUserToast(`Could not open the draft: ${e?.body ?? e?.message ?? e}`, true)
		}
	}
</script>

{#snippet lockedNote()}
	<div class="flex items-start gap-1.5 px-3 py-1 text-2xs text-secondary">
		<Lock size={11} class="mt-0.5 shrink-0" />
		<span>
			{draftsLockedReason}
			{#if draftsSignInHref}
				<a href={draftsSignInHref} class="font-medium text-accent hover:underline">Sign in</a>
			{/if}
		</span>
	</div>
{/snippet}

{#if edits.length > 0 || artifacts.length > 0}
	<div class="flex items-center gap-3 rounded-md border bg-surface-tertiary px-2 py-2 text-xs">
		{#if edits.length > 0}
			<SessionStatusPopover
				bind:open={editsOpen}
				label={token.label}
				title="Edited this session"
				items={edits}
				itemKey={(e) => `${e.kind}:${e.path}`}
				itemHref={deployedHref}
				{rowTitle}
				itemDisabled={(edit) => (edit.draft ? !opensDraft(edit) : !deployedHref(edit))}
				onPick={(edit) => void viewDraft(edit)}
				triggerClass={`${TOKEN_TRIGGER_CLASS} ${token.class}`}
				widthClass="w-96"
				maxHeightClass="max-h-[min(9rem,50vh)]"
				footer={draftsLockedReason && hasLockedRow ? lockedNote : undefined}
			>
				{#snippet row(edit)}
					<RowIcon kind={edit.kind as any} path={edit.path} size={14} />
					<span class="min-w-0 flex-1 truncate font-mono font-normal text-primary">
						{edit.path}
					</span>
					{#if edit.draft}
						<Badge small color="indigo">{edit.draftOnly ? 'Draft only' : 'Draft'}</Badge>
						{#if opensDraft(edit)}
							<GitCompareArrows size={12} class="shrink-0 text-tertiary" />
						{:else}
							<Lock size={12} class="shrink-0 text-tertiary" />
						{/if}
					{:else}
						<Badge small color="green">Deployed</Badge>
						{#if deployedHref(edit)}
							<ExternalLink size={12} class="shrink-0 text-tertiary" />
						{/if}
					{/if}
				{/snippet}
			</SessionStatusPopover>
		{/if}
		{#if artifacts.length > 0}
			<ArtifactsSegment readOnly={{ artifacts, open: openArtifact }} />
		{/if}
	</div>
{/if}

{#if diffDrawerModule.current}
	{@const LoadedDiffDrawer = diffDrawerModule.current.default}
	<LoadedDiffDrawer bind:this={diffDrawer} />
{/if}
