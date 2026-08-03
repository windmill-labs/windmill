<script lang="ts">
	import { base } from '$app/paths'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { WorkerService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { resource } from 'runed'
	import { hasWorkerForTag } from './missingWorker'

	interface Props {
		/** Worker tag the feature's jobs run on. */
		tag: string
		/** What breaks without it, used as the sentence subject, e.g. "Data table queries". */
		subject: string
		workspace?: string
		class?: string
	}

	let { tag, subject, workspace = undefined, class: className = '' }: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	// With per-workspace default tags, jobs run on a workspace-suffixed variant of
	// `tag`, so probing the bare tag would warn about a tag nothing uses. The
	// in-flight check in `pollJobResult` reads the job's real tag and still covers
	// those instances.
	const served = resource(
		() => [ws, tag] as const,
		async ([ws, tag]) => {
			if (!ws || (await WorkerService.isDefaultTagsPerWorkspace())) return true
			return await hasWorkerForTag(ws, tag)
		}
	)
</script>

{#if served.current === false}
	<div class={className}>
		<!-- Deliberately not phrased as "this tag is not configured": a correctly
			tagged group sitting at zero replicas is indistinguishable from one in
			`worker_ping`, and queueing a job is what scales it back up. -->
		<Alert type="warning" title="No worker is running for the &quot;{tag}&quot; tag" size="xs">
			{subject} run as Windmill jobs tagged <b>{tag}</b>, and no worker is currently listening to
			that tag, so they stay queued until one is. If no worker group is meant to serve it, add
			<b>{tag}</b>
			to a group's worker tags on the <a href="{base}/workers">workers page</a>.
		</Alert>
	</div>
{/if}
