<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { WorkerService } from '$lib/gen'
	import { resource } from 'runed'
	import { hasWorkerForTag } from './missingWorker'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { userWorkspaces } from '$lib/stores'
	import {
		devWorkspaceTagSuffix,
		isIsolatedWorkspaceId,
		loadDevWorkspaceTagSuffix,
		withDevSuffix
	} from '$lib/devWorkspaceTags'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		/** Worker tag the feature's jobs run on. */
		tag: string
		/** What breaks without it, used as the sentence subject, e.g. "Data table queries". */
		subject: string
		workspace?: string
		class?: string
	}

	let { tag, subject, workspace = undefined, class: className = '' }: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)

	// With per-workspace default tags, jobs run on a workspace-suffixed variant of
	// `tag`, so probing the bare tag would warn about a tag nothing uses. The
	// in-flight check in `pollJobResult` reads the job's real tag and still covers
	// those instances.
	// A dev workspace's jobs run on the suffixed tag, which only groups serving dev workspaces pull.
	let devSuffix = $derived(
		$devWorkspaceTagSuffix && isIsolatedWorkspaceId(ws, $userWorkspaces)
			? $devWorkspaceTagSuffix
			: undefined
	)
	let runTag = $derived(devSuffix ? withDevSuffix(tag, devSuffix) : tag)
	const served = resource(
		() => [ws, runTag] as const,
		async ([ws, runTag]) => {
			await loadDevWorkspaceTagSuffix()
			if (!ws || (await WorkerService.isDefaultTagsPerWorkspace())) return true
			return await hasWorkerForTag(ws, runTag)
		}
	)
</script>

{#if served.current === false && devSuffix}
	<div class={className}>
		<Alert type="warning" title="No worker is running for the &quot;{runTag}&quot; tag" size="xs">
			{subject} run as Windmill jobs tagged <b>{tag}</b>, which this workspace, a dev workspace or a
			fork, runs as <b>{runTag}</b>. No worker is currently listening to it, so they stay queued
			until one is. An admin can serve it by flagging a worker group that lists <b>{tag}</b> as
			serving dev workspaces on the <a href="/workers">workers page</a>.
		</Alert>
	</div>
{:else if served.current === false}
	<div class={className}>
		<!-- Deliberately not phrased as "this tag is not configured": a correctly
			tagged group sitting at zero replicas is indistinguishable from one in
			`worker_ping`, and queueing a job is what scales it back up. -->
		<Alert type="warning" title="No worker is running for the &quot;{tag}&quot; tag" size="xs">
			{subject} run as Windmill jobs tagged <b>{tag}</b>, and no worker is currently listening to
			that tag, so they stay queued until one is. If no worker group is meant to serve it, add
			<b>{tag}</b>
			to a group's worker tags on the <a href="/workers">workers page</a>.
		</Alert>
	</div>
{/if}
