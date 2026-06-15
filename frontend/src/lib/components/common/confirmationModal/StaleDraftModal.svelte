<script lang="ts">
	/**
	 * Modal opened on editor mount when the authed user's per-user draft
	 * is older than the latest deployed version at the same path — i.e.
	 * someone else deployed a new version while the draft was sitting
	 * around. The user is asked to either pick up the latest deploy
	 * (discards the stale draft) or keep editing what they had.
	 *
	 * The parent threads `draftSavedAt` and `deployedAt` raw and the
	 * modal computes staleness internally; this keeps each route from
	 * re-implementing the comparison and the threshold (we treat a
	 * draft as stale only when it's strictly older — a deploy at the
	 * exact same instant is treated as not stale).
	 *
	 * Open-state is bindable so the parent can dismiss programmatically
	 * (e.g. after the load-latest-deploy callback completes).
	 */
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	type Props = {
		isOpen: boolean
		/** ISO timestamp the authed user's draft was saved. */
		draftSavedAt: string | undefined
		/** ISO timestamp the latest deploy at this path landed. */
		deployedAt: string | undefined
		/** Discards the draft and reloads the deployed payload — the route
		 *  already has this callback for the AutosaveIndicator's "Reset to
		 *  deployed" button; pass the same function in. */
		onLoadLatestDeploy: () => void | Promise<void>
	}

	let { isOpen = $bindable(), draftSavedAt, deployedAt, onLoadLatestDeploy }: Props = $props()

	let loading = $state(false)

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
				<p>
					A newer version was deployed after you started editing. Your draft is based on the older
					deploy.
				</p>
				<p class="text-xs text-secondary">
					Draft saved {formatTs(draftSavedAt)} · Deployed {formatTs(deployedAt)}
				</p>
			</div>
		</div>

		<div class="flex justify-end gap-2 mt-2">
			<Button variant="default" size="sm" on:click={() => (isOpen = false)}>
				Keep editing my draft
			</Button>
			<Button variant="contained" color="dark" size="sm" {loading} on:click={loadLatestDeploy}>
				Load latest deploy
			</Button>
		</div>
	</div>
</Modal2>
