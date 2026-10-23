<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { jobsPause } from './jobsPause.svelte'

	const durations = [
		{ label: '15 minutes', value: 15 * 60 },
		{ label: '30 minutes', value: 30 * 60 },
		{ label: '1 hour', value: 60 * 60 },
		{ label: '2 hours', value: 2 * 60 * 60 },
		{ label: '6 hours', value: 6 * 60 * 60 },
		{ label: '1 day', value: 24 * 60 * 60 },
		{ label: '7 days', value: 7 * 24 * 60 * 60 }
	]

	let durationS = $state(30 * 60)
	let reason = $state('')
	let busy = $state(false)

	let pause = $derived(jobsPause.workspace === $workspaceStore ? jobsPause.pause : null)

	async function pauseJobs() {
		const workspace = $workspaceStore
		if (!workspace) return
		busy = true
		try {
			const p = await WorkspaceService.pauseJobs({
				workspace,
				requestBody: { duration_s: durationS, reason: reason || undefined, source: 'ui' }
			})
			jobsPause.set(workspace, p)
			sendUserToast('New jobs are paused')
		} catch (e) {
			sendUserToast(`Could not pause jobs: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function resumeJobs() {
		const workspace = $workspaceStore
		if (!workspace) return
		busy = true
		try {
			const released = await WorkspaceService.resumeJobs({ workspace })
			jobsPause.set(workspace, null)
			sendUserToast(`Jobs resumed, ${released} held job${released === 1 ? '' : 's'} released`)
		} catch (e) {
			sendUserToast(`Could not resume jobs: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}
</script>

<div class="flex flex-col gap-1">
	<p class="font-semibold text-xs text-emphasis">Pause new jobs</p>
	<p class="text-xs text-secondary font-normal max-w-2xl">
		Hold the jobs that triggers, schedules and webhooks start, for example while a deploy installs
		dependencies or migrations are pending. Held jobs run when jobs resume, at the latest when the
		pause ends. Callers waiting for a result, such as apps and synchronous routes, get an error
		instead. Dependency and git sync jobs, steps of flows already running and runs started by an
		admin are not held.
	</p>
	{#if pause}
		<div class="flex flex-row gap-2 items-center mt-1">
			<span class="text-xs text-primary">
				Paused until {displayDate(pause.paused_until)} by {pause.paused_by}, {pause.held_count}
				held job{pause.held_count === 1 ? '' : 's'}
				{#if pause.reason}({pause.reason}){/if}
			</span>
			<Button variant="accent" unifiedSize="sm" loading={busy} onclick={resumeJobs}>
				Resume jobs
			</Button>
		</div>
	{:else}
		<div class="flex flex-row gap-2 items-center mt-1 flex-wrap">
			<Select items={durations} bind:value={durationS} size="sm" class="w-36" />
			<TextInput
				bind:value={reason}
				size="sm"
				class="max-w-xs"
				inputProps={{ placeholder: 'Reason (optional)', 'aria-label': 'Pause reason' }}
			/>
			<Button variant="default" unifiedSize="sm" loading={busy} onclick={pauseJobs}>
				Pause jobs
			</Button>
		</div>
	{/if}
</div>
