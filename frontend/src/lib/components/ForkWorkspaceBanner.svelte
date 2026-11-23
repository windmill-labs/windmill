<script lang="ts">
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { AlertTriangle, ArrowDown, ArrowUp, Loader2, Pencil } from 'lucide-svelte'
	import {
		roleCapabilities,
		workspaceStore,
		userWorkspaces,
		userStore,
		type UserExt
	} from '$lib/stores'
	import type { WorkspaceComparison } from '$lib/gen'
	import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
	import { Button } from './common'
	import { onDestroy, onMount, untrack } from 'svelte'
	import {
		homeActionsGathered,
		homeCountLabels,
		homeHeaderMenu
	} from './home/homeHeaderMenu.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { childWorkspaceNoun } from '$lib/utils/devWorkspaceLabel'
	import { diffActionableInDirection } from '$lib/utils_workspace_deploy'

	let loading = $state(false)
	let comparison: WorkspaceComparison | undefined = $state(undefined)
	/** Workspace `comparison` describes; control flow only, never rendered. */
	let comparisonFor: string | undefined = undefined
	let requestSeq = 0
	let error: string | undefined = $state(undefined)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)
	// Detect fork/dev workspaces by their parent link, not the `wm-fork-` id prefix (dev
	// workspaces have an ordinary, prefix-less id). Keying on the parent (rather than the
	// prefix) also avoids a parentless "Fork of ()" banner when the linkage is dropped.
	let isFork = $derived(parentWorkspaceId != null)
	let currentNoun = $derived(childWorkspaceNoun(currentWorkspaceData))
	// A role without `code` never deploys a fork, so the banner and its CTA are noise
	// for it. Gates the fetches too, not just the markup: the fork/parent comparison
	// is an expensive tally such a role cannot act on.
	//
	// Only a user fetched for the workspace we are on answers this. `$workspaceStore`
	// flips synchronously on a switch while `$userStore` still holds the workspace we
	// left, so trusting it unqualified would flash the banner at (and start the tally
	// for) a restricted member entering a fork from a workspace where they are not one.
	function isUserFor(user: UserExt | undefined, ws: string | undefined): boolean {
		return !!user && !!ws && user.workspace_id === ws
	}
	let isNotOperator = $derived(isUserFor($userStore, $workspaceStore) && $roleCapabilities.code)
	let showBanner = $derived(isFork && isNotOperator)

	// Drafts in this fork. When the fork is otherwise in sync with its parent, a
	// user with only pending drafts should still get the draft CTA (mirrors the
	// non-fork WorkspaceDraftsBanner). Pass undefined when the banner is hidden so
	// it doesn't fetch.
	const drafts = useWorkspaceDrafts(() => (showBanner ? ($workspaceStore ?? undefined) : undefined))
	const draftCount = $derived(drafts.count)

	// Every read of `comparison` that decides what the banner says or where its button
	// goes must go through this: anything else — in flight, failed, or a tally skipped
	// outright — counts zero of everything, which is indistinguishable from "nothing to
	// deploy". Typed helper avoids the `never`-inference quirk on `$state` in `$derived`.
	function isAnswerable(c: WorkspaceComparison | undefined, isLoading: boolean): boolean {
		return !isLoading && !!c && !c.skipped_comparison
	}
	const hasAnswer = $derived(isAnswerable(comparison, loading))

	const compactCounts = $derived(!homeCountLabels())

	// Fork is fully in sync with its parent (comparison ran, no ahead/behind diffs).
	function isUpToDate(c: WorkspaceComparison | undefined): boolean {
		return !!c && !c.skipped_comparison && c.summary.total_diffs === 0
	}
	let upToDate = $derived(hasAnswer && isUpToDate(comparison))

	// Leaving for a workspace with no comparison of its own has to invalidate whatever
	// is in flight too, or that response lands as this one's answer.
	function dropComparison() {
		requestSeq++
		comparison = undefined
		comparisonFor = undefined
		loading = false
	}

	// `isNotOperator` is a dependency of its own: it only turns true once this
	// workspace's role has landed, which is after the switch that triggered it.
	$effect(() => {
		;[$workspaceStore, parentWorkspaceId, isNotOperator]
		untrack(() => {
			if (showBanner && $workspaceStore) {
				checkForChanges()
			} else {
				dropComparison()
			}
		})
	})

	onMount(() => {
		if (showBanner && $workspaceStore) {
			checkForChanges()
		} else {
			dropComparison()
		}
	})

	async function checkForChanges() {
		const ws = $workspaceStore
		const parent = parentWorkspaceId
		if (!ws || !parent) {
			return
		}

		// A comparison only ever describes the workspace it was requested for. The
		// component survives a fork switch, so drop the previous fork's rows before
		// fetching rather than let them answer for this one, and let only the newest
		// request write — responses can land out of order, and a late one would
		// otherwise paint another fork's counts over the current answer.
		if (comparisonFor !== ws) {
			comparison = undefined
			comparisonFor = undefined
		}
		const seq = ++requestSeq
		loading = true
		error = undefined

		try {
			// Compare with parent workspace (shared single-flight fetch — the chat
			// diff tool reuses this result instead of recomputing the comparison)
			const result = await fetchWorkspaceComparison(parent, ws)

			if (seq !== requestSeq) return
			comparison = result
			comparisonFor = ws
		} catch (e) {
			if (seq !== requestSeq) return
			console.error('Failed to compare workspaces:', e)
			error = `Failed to check for changes: ${e}`
			// Show the banner with the error rather than the rows we failed to refresh:
			// on a switch those belong to the fork we just left.
			comparison = undefined
			comparisonFor = undefined
		} finally {
			if (seq === requestSeq) loading = false
		}
	}

	/** The compare page opened on one direction. */
	function compareDirHref(dir: 'deploy_to' | 'update'): string | undefined {
		if (!parentWorkspaceId || !$workspaceStore) return undefined
		return (
			'/forks/compare?workspace_id=' +
			encodeURIComponent($workspaceStore) +
			(dir === 'update' ? '&dir=update' : '')
		)
	}
	const draftCompareHref = $derived(
		$workspaceStore
			? '/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore) + '&mode=draft'
			: undefined
	)

	// Counted with the compare page's own predicate so the banner never advertises a
	// direction whose list is empty: the `ahead`/`behind` sums in the summary include
	// rows a direction does not carry, and miss a parent-only row that the update
	// direction carries at `behind = 0`.
	function countDir(c: WorkspaceComparison | undefined, mergeIntoParent: boolean): number {
		return c?.diffs.filter((d) => diffActionableInDirection(d, mergeIntoParent)).length ?? 0
	}
	const changesAhead = $derived(countDir(comparison, true))
	const changesBehind = $derived(countDir(comparison, false))
	// Opens the direction the button offers, so the label and the list agree: a fork
	// with nothing to deploy lands on the update side, not on an empty deploy list.
	const comparisonHref = $derived(
		compareDirHref(hasAnswer && changesAhead === 0 ? 'update' : 'deploy_to')
	)

	function countConflicts(c: WorkspaceComparison | undefined): number {
		return c?.summary.conflicts ?? 0
	}
	const conflicts = $derived(hasAnswer ? countConflicts(comparison) : 0)
	// Every count opens the same compare page, on its own tab. Drafts get their tab only when they
	// are the only change: the other tabs show the drafts count in their toggle and flag the rows
	// that carry one.
	const changesHref = $derived(
		hasAnswer && changesAhead === 0 && changesBehind === 0 && draftCount > 0
			? draftCompareHref
			: comparisonHref
	)
	const countParts = $derived(
		[
			hasAnswer && changesAhead > 0 ? `${changesAhead} ahead of ${parentWorkspaceId}` : '',
			hasAnswer && changesBehind > 0 ? `${changesBehind} behind` : '',
			conflicts > 0 ? `${conflicts} conflict${conflicts !== 1 ? 's' : ''}` : '',
			draftCount > 0 ? `${draftCount} draft${draftCount !== 1 ? 's' : ''}` : ''
		].filter(Boolean)
	)

	// Folded into Home's ⋮ menu, the button is one row that says what it counted.
	const menuId = homeHeaderMenu.register(0, () => {
		if (!showBanner || !homeActionsGathered()) return []
		return [
			{
				displayName: 'Review changes',
				description: countParts.length > 0 ? countParts.join(' · ') : undefined,
				href: changesHref
			}
		]
	})
	onDestroy(() => homeHeaderMenu.release(menuId))
</script>

{#if showBanner && !homeActionsGathered()}
	<!-- In the band, not a banner: the breadcrumb already says which fork this is, so all this has
	     to carry is how far it has drifted and the way to review that. The per-kind breakdown, the
	     CI line and the conflict count live on the compare page the button opens. -->
	<PageHeaderContent actions={forkAction} />
{/if}

{#snippet forkAction()}
	<!-- One button for every count, since they all open the compare page; a count of its own
	     beside it would read as a second way in. The colours stay on the icons, the ones the
	     compare page gives these kinds. A conflict is ahead and behind at once, so it has no arrow
	     of its own; drafts are work not deployed anywhere yet, told apart by the pencil. -->
	<!-- A failed comparison still shows the drafts count, so the name keeps it beside the error. -->
	{@const label = [
		error ? `Comparison failed: ${error}` : undefined,
		countParts.length > 0
			? `${countParts.join(', ')}: review changes`
			: `Review this ${currentNoun} against ${parentWorkspaceId}`
	]
		.filter(Boolean)
		.join('. ')}
	<!-- Named by aria-label too: compact, the button's own text is bare numbers. -->
	<Button
		variant="default"
		unifiedSize="sm"
		href={changesHref}
		btnClasses="tabular-nums"
		title={label}
		aria-label={label}
	>
		<span class="flex items-center gap-2.5 wm-page-in">
			{#if loading}
				<Loader2 class="w-3 h-3 animate-spin text-hint" />
			{:else if error}
				<AlertTriangle class="w-3 h-3 text-red-600 dark:text-red-400" />
			{/if}
			{#if hasAnswer && changesAhead > 0}
				<span class="flex items-center gap-1">
					<ArrowUp class="w-3 h-3 text-green-600 dark:text-green-400" />
					{changesAhead}{compactCounts ? '' : ' ahead'}
				</span>
			{/if}
			{#if hasAnswer && changesBehind > 0}
				<span class="flex items-center gap-1">
					<ArrowDown class="w-3 h-3 text-blue-600 dark:text-blue-400" />
					{changesBehind}{compactCounts ? '' : ' behind'}
				</span>
			{/if}
			{#if conflicts > 0}
				<span class="flex items-center gap-1">
					<AlertTriangle class="w-3 h-3 text-orange-600 dark:text-orange-400" />
					{conflicts}{compactCounts ? '' : ` conflict${conflicts !== 1 ? 's' : ''}`}
				</span>
			{/if}
			{#if draftCount > 0}
				<span class="flex items-center gap-1">
					<Pencil class="w-3 h-3 text-accent" />
					{draftCount}{compactCounts ? '' : ` draft${draftCount !== 1 ? 's' : ''}`}
				</span>
			{/if}
			{#if !hasAnswer && draftCount === 0}
				Compare
			{:else if upToDate && draftCount === 0}
				Up to date
			{/if}
		</span>
	</Button>
{/snippet}
