<script lang="ts">
	import { Button } from '$lib/components/common'
	import { classes as alertClasses, icons as alertIcons } from '$lib/components/common/alert/model'
	import { WorkspaceService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate } from '$lib/utils'
	import { jobsPause } from './jobsPause.svelte'
	import { untrack } from 'svelte'

	// A pause can be started from the CLI or a flow, so the banner polls rather than waiting
	// for this tab to change it.
	const POLL_MS = 30_000

	$effect(() => {
		const workspace = $workspaceStore
		if (!workspace) return
		// `load` reads and writes the shared state; only the workspace should re-run this effect.
		untrack(() => jobsPause.load(workspace))
		const interval = setInterval(() => jobsPause.load(workspace), POLL_MS)
		return () => clearInterval(interval)
	})

	let resuming = $state(false)
	async function resume() {
		const workspace = $workspaceStore
		if (!workspace) return
		resuming = true
		try {
			const released = await WorkspaceService.resumeJobs({ workspace })
			jobsPause.set(workspace, null)
			sendUserToast(`Jobs resumed, ${released} held job${released === 1 ? '' : 's'} released`)
		} catch (e) {
			sendUserToast(`Could not resume jobs: ${e?.body ?? e}`, true)
		} finally {
			resuming = false
		}
	}

	let pause = $derived(jobsPause.workspace === $workspaceStore ? jobsPause.pause : null)
	const palette = alertClasses.warning
	const Icon = alertIcons.warning
</script>

{#if pause}
	<div
		class="shrink-0 px-4 py-1.5 flex items-center justify-center gap-x-3 gap-y-1 flex-wrap {palette.bgClass}"
		role="status"
	>
		<Icon size={16} class="shrink-0 {palette.iconClass}" />
		<span class="text-xs font-medium {palette.titleClass}">
			New jobs are paused until {displayDate(pause.paused_until)}
			{#if pause.held_count > 0}
				· {pause.held_count} held
			{/if}
			{#if pause.reason}
				· {pause.reason}
			{/if}
		</span>
		{#if $userStore?.is_admin || $userStore?.is_super_admin}
			<Button variant="subtle" unifiedSize="xs" loading={resuming} onclick={resume}>
				Resume jobs
			</Button>
		{/if}
	</div>
{/if}
