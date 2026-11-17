<script lang="ts" module>
	import { lazyModule } from '$lib/lazyModule.svelte'

	const diffDrawerModule = lazyModule(() => import('$lib/components/DiffDrawer.svelte'))
</script>

<script lang="ts">
	import { tick } from 'svelte'
	import { ExternalLink, GitCompareArrows } from 'lucide-svelte'
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
		/** The author's username, when the reader is signed in to their workspace and may ask
		 *  for the author's drafts. */
		draftAuthor: string | undefined
		artifacts: PersistedArtifact[]
		openArtifact: (artifact: PersistedArtifact) => void
	}

	let { edits, editsWorkspace, draftAuthor, artifacts, openArtifact }: Props = $props()

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
			edit.draft &&
			!edit.draftOnly &&
			!!draftAuthor &&
			!!editsWorkspace &&
			!!edit.draftKind &&
			!!edit.draftPath &&
			SHARED_DRAFT_KINDS.has(edit.draftKind)
		)
	}

	function rowTitle(edit: SharedSessionEdit): string {
		if (!edit.draft) return deployedHref(edit) ? `Open ${edit.path}` : edit.path
		if (opensDraft(edit)) return `View ${draftAuthor}'s draft of ${edit.path}`
		return `${edit.path}: a draft only its author can open`
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
					username: draftAuthor
				}),
				fetchDeployedValueForDiff(workspace, kind, edit.path),
				diffDrawerModule.load()
			])
			await tick()
			diffDrawer?.openDrawer()
			diffDrawer?.setDiff({
				mode: 'simple',
				title: `${draftAuthor}'s draft vs deployed`,
				original: deployed ?? {},
				current: draft.value as any
			})
		} catch (e: any) {
			sendUserToast(`Could not open the draft: ${e?.body ?? e?.message ?? e}`, true)
		}
	}
</script>

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
				onPick={(edit) => void viewDraft(edit)}
				triggerClass={`${TOKEN_TRIGGER_CLASS} ${token.class}`}
				widthClass="w-96"
				maxHeightClass="max-h-[min(9rem,50vh)]"
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
