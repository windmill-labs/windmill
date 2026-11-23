<script lang="ts">
	import { ConfigService, SettingsService } from '$lib/gen'
	import { Download, Circle, Square } from 'lucide-svelte'
	import { Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import { displayDate, download } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { untrack } from 'svelte'

	interface Props {
		worker_group: string
		/** Bumped by the panel's refresh button to reload the recording state. */
		refreshKey?: number
	}

	let { worker_group, refreshKey }: Props = $props()

	let recordingUntil: string | undefined = $state(undefined)
	let decisionCount = $state(0)
	let busy = $state(false)

	async function loadState() {
		try {
			const diagnostics = await ConfigService.getAutoscalingDiagnostics({
				workerGroup: worker_group,
				includeDecisions: false
			})
			recordingUntil = diagnostics.recording_until ?? undefined
			decisionCount = diagnostics.decision_count
		} catch (e) {
			console.error(e)
		}
	}

	$effect(() => {
		refreshKey
		worker_group && untrack(() => loadState())
	})

	// Decisions arrive every 15-20s while recording: keep the count, and so Download, current.
	$effect(() => {
		if (!recordingUntil) return
		const interval = setInterval(() => loadState(), 30_000)
		return () => clearInterval(interval)
	})

	async function setRecording(record: boolean) {
		busy = true
		try {
			await ConfigService.setAutoscalingRecording({
				workerGroup: worker_group,
				requestBody: { record }
			})
			if (record) {
				logFeatureUsage('autoscaling_diagnostics', 'recording_started')
				sendUserToast(
					'Recording autoscaling decisions for 24h. They are kept for 48h, download them from here.'
				)
			}
			await loadState()
		} catch (e) {
			sendUserToast(`Could not change the recording: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function downloadDiagnostics() {
		busy = true
		try {
			const [diagnostics, config, version] = await Promise.all([
				ConfigService.getAutoscalingDiagnostics({ workerGroup: worker_group }),
				ConfigService.getConfig({ name: `worker__${worker_group}` }).catch(() => undefined),
				SettingsService.backendVersion().catch(() => undefined)
			])
			const payload = {
				worker_group,
				version,
				exported_at: new Date().toISOString(),
				autoscaling: (config as { autoscaling?: unknown } | undefined)?.autoscaling,
				...diagnostics
			}
			download(
				`autoscaling-${worker_group}-${new Date().toISOString().slice(0, 16).replace(':', '-')}.json`,
				JSON.stringify(payload, null, 2),
				'application/json'
			)
			logFeatureUsage('autoscaling_diagnostics', 'downloaded')
		} catch (e) {
			sendUserToast(`Could not download the diagnostics: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}
</script>

<div class="flex flex-row items-center gap-2">
	{#if recordingUntil}
		<span class="text-2xs text-secondary">Recording until {displayDate(recordingUntil)}</span>
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: Square }}
			disabled={busy}
			onClick={() => setRecording(false)}
		>
			Stop
		</Button>
	{:else}
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: Circle }}
			disabled={busy}
			onClick={() => setRecording(true)}
			title="Record every autoscaling decision of this worker group, including deciding not to scale, for 24h"
		>
			Record decisions
		</Button>
	{/if}
	<Button
		variant="subtle"
		unifiedSize="sm"
		startIcon={{ icon: Download }}
		disabled={busy || decisionCount == 0}
		onClick={downloadDiagnostics}
		title={decisionCount == 0
			? 'No decisions recorded in the last 48h'
			: `Download the ${decisionCount} decisions recorded in the last 48h`}
	>
		Download
	</Button>
</div>
