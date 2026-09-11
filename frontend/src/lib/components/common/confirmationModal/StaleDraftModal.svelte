<script lang="ts">
	/**
	 * The prompt for a draft that is behind: someone deployed a newer version
	 * of the item after the draft forked from it. Opened on every load while
	 * that holds (the parent computes it; see DraftEditorModals), it names the
	 * two versions and offers two ways out: look at the diff, or drop the draft
	 * for the latest deploy. Closing it is "keep editing". Taking the latest as
	 * the new base while keeping the edits lives in the diff drawer, where the
	 * user can see what they are taking.
	 *
	 * Open-state is bindable so the parent can dismiss programmatically
	 * (e.g. after the load-latest-deploy callback completes).
	 */
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertTriangle, GitCompare } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import type { UserDraftItemKind } from '$lib/gen'

	type Props = {
		isOpen: boolean
		itemKind: UserDraftItemKind
		/** ISO timestamp the authed user's draft was saved. Shown only when the
		 *  versions below are unknown (a draft that predates the base). */
		draftSavedAt: string | undefined
		/** ISO timestamp the latest deploy at this path landed. */
		deployedAt: string | undefined
		/** The version the draft forked from and the deployed head, as text. */
		draftBaseVersion?: string | undefined
		deployedHeadVersion?: string | undefined
		/** Who deployed the head. */
		deployedBy?: string | undefined
		/** Discards the draft and reloads the deployed payload — the route
		 *  already has this callback for the AutosaveIndicator's "Reset to
		 *  deployed" button; pass the same function in. */
		onLoadLatestDeploy: () => void | Promise<void>
		/** Opens the editor's own Deployed↔Current diff. Without it the user is
		 *  asked to choose between keeping and discarding their draft with no way
		 *  to see what actually differs — and after a rename the difference is
		 *  often only the path. Omitted where the editor has no diff drawer. */
		onViewDiff?: () => void | Promise<void>
	}

	let {
		isOpen = $bindable(),
		itemKind,
		draftSavedAt,
		deployedAt,
		draftBaseVersion = undefined,
		deployedHeadVersion = undefined,
		deployedBy = undefined,
		onLoadLatestDeploy,
		onViewDiff
	}: Props = $props()

	let loading = $state(false)

	// Scripts are versioned by hash, the other kinds by a numeric version id;
	// the diff picker renders them the same way.
	function formatVersion(v: string): string {
		return itemKind === 'script' ? v.slice(0, 8) : `v${v}`
	}

	async function loadLatestDeploy() {
		if (loading) return
		loading = true
		try {
			await onLoadLatestDeploy()
			isOpen = false
		} catch (e: any) {
			sendUserToast(`Could not load latest deploy: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			loading = false
		}
	}

	// Dismisses on the way out: the diff drawer opens behind this modal, so
	// leaving it up would cover the thing the user asked to see.
	async function viewDiff() {
		isOpen = false
		await onViewDiff?.()
	}

	function formatTs(ts: string | undefined): string {
		if (!ts) return ''
		try {
			return new Date(ts).toLocaleString()
		} catch {
			return ts
		}
	}
</script>

<Modal2 bind:isOpen title="Your draft is out of date" fixedWidth="sm" fixedHeight="adaptive">
	<div class="flex flex-col w-full gap-4">
		<div class="flex gap-3 items-start">
			<AlertTriangle size={20} class="text-amber-500 shrink-0 mt-0.5" />
			<div class="flex flex-col gap-1 text-sm text-primary">
				<p>A newer version was deployed after you started editing.</p>
				{#if draftBaseVersion && deployedHeadVersion}
					<p class="text-xs text-secondary">
						Your draft is based on <span class="font-mono">{formatVersion(draftBaseVersion)}</span>
						· latest is <span class="font-mono">{formatVersion(deployedHeadVersion)}</span>
						{#if deployedBy}by {deployedBy}{/if}{#if deployedAt}, {formatTs(deployedAt)}{/if}
					</p>
				{:else}
					<p class="text-xs text-secondary">
						Draft saved {formatTs(draftSavedAt)} · Deployed {formatTs(deployedAt)}
					</p>
				{/if}
			</div>
		</div>

		<div class="flex justify-between items-center gap-2 mt-2">
			{#if onViewDiff}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: GitCompare }}
					on:click={viewDiff}
				>
					See what changed
				</Button>
			{:else}
				<div></div>
			{/if}
			<!-- Red: this replaces the draft with the latest deploy. Closing the modal
			     is "keep editing", so it needs no button of its own. -->
			<Button variant="accent" destructive unifiedSize="sm" {loading} on:click={loadLatestDeploy}>
				Use latest
			</Button>
		</div>
	</div>
</Modal2>
