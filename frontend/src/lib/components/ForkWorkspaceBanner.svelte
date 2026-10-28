<script lang="ts">
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import { Badge, Button } from './common'
	import { AlertTriangle, ArrowDown, ArrowUp, Pencil } from 'lucide-svelte'
	import {
		roleCapabilities,
		workspaceStore,
		userWorkspaces,
		userStore,
		type UserExt
	} from '$lib/stores'
	import type { WorkspaceComparison } from '$lib/gen'
	import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
	import { goto } from '$app/navigation'
	import { onMount, untrack } from 'svelte'
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

	// Drafts in this fork. Pass undefined when the banner is hidden so it doesn't fetch.
	const drafts = useWorkspaceDrafts(() => (showBanner ? ($workspaceStore ?? undefined) : undefined))
	const draftCount = $derived(drafts.count)

	/** Bar width from which each badge says what it counts. Below it they keep the icon and the
	 *  number, which is the part a reader scans for; the tooltip names them either way. */
	const BAR_FITS_COUNT_LABELS = 1280
	const compactCounts = $derived(pageHeader.barWidth < BAR_FITS_COUNT_LABELS)

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

	/** The compare page opened on one direction, for a badge that counts exactly that. */
	function openCompareDir(dir: 'deploy_to' | 'update') {
		if (!parentWorkspaceId || !$workspaceStore) return
		goto(
			'/forks/compare?workspace_id=' +
				encodeURIComponent($workspaceStore) +
				(dir === 'update' ? '&dir=update' : ''),
			{ replaceState: true }
		)
	}

	function openDraftCompare() {
		if ($workspaceStore) {
			goto('/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore) + '&mode=draft', {
				replaceState: true
			})
		}
	}

	// Counted with the compare page's own predicate so the banner never advertises a
	// direction whose list is empty: the `ahead`/`behind` sums in the summary include
	// rows a direction does not carry, and miss a parent-only row that the update
	// direction carries at `behind = 0`.
	function countDir(c: WorkspaceComparison | undefined, mergeIntoParent: boolean): number {
		return c?.diffs.filter((d) => diffActionableInDirection(d, mergeIntoParent)).length ?? 0
	}
	const changesAhead = $derived(countDir(comparison, true))
	const changesBehind = $derived(countDir(comparison, false))
	const conflicts = $derived(comparison?.summary.conflicts ?? 0)
	// The badges are the only way to the compare page, so with none to show (in sync, still
	// counting, or the count failed) the button takes their place.
	const showsBadge = $derived(
		!loading && !error && (changesAhead > 0 || changesBehind > 0 || conflicts > 0 || draftCount > 0)
	)

	function openCompare() {
		if ($workspaceStore) {
			goto('/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore), {
				replaceState: true
			})
		}
	}
</script>

{#if showBanner}
	<!-- In the band, not a banner: the breadcrumb already says which fork this is, so all this has
	     to carry is how far it has drifted, as badges that each open the compare page on what they
	     count. The per-kind breakdown and the CI line live on that page. -->
	<PageHeaderContent actions={forkAction} />
{/if}

{#snippet forkAction()}
	{#if loading}
		<span class="text-2xs text-tertiary">Checking for changes…</span>
	{:else if error}
		<span class="text-2xs text-red-600 dark:text-red-400" title={error}>Comparison failed</span>
	{:else}
		<!-- Which way the drift goes is the thing to know at a glance: ahead is what this fork has
		     to give its parent, behind is what it has yet to take. -->
		<!-- The colours and the icons the compare page gives these three, so the counts in the bar
		     and the rows on that page say the same thing the same way. -->
		{#if changesAhead > 0}
			<Badge
				color="green"
				small
				clickable
				onclick={() => openCompareDir('deploy_to')}
				title="{changesAhead} ahead of {parentWorkspaceId} — review what this {currentNoun} has to give"
			>
				<ArrowUp class="w-3 h-3 inline" />
				{changesAhead}{compactCounts ? '' : ' ahead'}
			</Badge>
		{/if}
		{#if changesBehind > 0}
			<Badge
				color="blue"
				small
				clickable
				onclick={() => openCompareDir('update')}
				title="{changesBehind} behind {parentWorkspaceId} — review what this {currentNoun} has yet to take"
			>
				<ArrowDown class="w-3 h-3 inline" />
				{changesBehind}{compactCounts ? '' : ' behind'}
			</Badge>
		{/if}
		{#if conflicts > 0}
			<!-- A conflict is ahead and behind at once, so it has no direction of its own; it opens
			     the side the reader resolves it from before deploying. -->
			<Badge
				color="orange"
				small
				clickable
				onclick={() => openCompareDir('deploy_to')}
				title="{conflicts} conflicting item{conflicts !== 1 ? 's' : ''} — review them"
			>
				<AlertTriangle class="w-3 h-3 inline" />
				{conflicts}{compactCounts ? '' : ` conflict${conflicts !== 1 ? 's' : ''}`}
			</Badge>
		{/if}
		<!-- Drafts are a different axis from the drift beside them: ahead and behind count deployed
		     items against the parent, this counts work in this fork that is not deployed anywhere
		     yet. It shares blue with "behind" and is told apart by the pencil and by naming what it
		     counts, rather than by an arrow that would read as a third direction. Clicking goes to
		     the drafts half of the compare page. -->
		{#if draftCount > 0}
			<Badge
				color="blue"
				small
				clickable
				onclick={openDraftCompare}
				title="{draftCount} draft{draftCount !== 1
					? 's'
					: ''} in this {currentNoun}, not deployed anywhere yet — review and deploy"
			>
				<Pencil class="w-3 h-3 inline" />
				{draftCount}{compactCounts ? '' : ` draft${draftCount !== 1 ? 's' : ''}`}
			</Badge>
		{/if}
	{/if}
	{#if !showsBadge}
		<Button
			variant="subtle"
			unifiedSize="sm"
			onclick={openCompare}
			title="Review this {currentNoun} against {parentWorkspaceId}"
		>
			Compare with {parentWorkspaceId}
		</Button>
	{/if}
{/snippet}
