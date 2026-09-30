<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import PermissionedAsLine from '$lib/components/triggers/PermissionedAsLine.svelte'
	import type { AgentDraftHandle, AgentRunAs } from '../agentDraft.svelte'

	interface Props {
		draft: AgentDraftHandle
		/** The path the editor opened, which a first deploy of a new agent replaces. */
		path: string
		workspace: string | undefined
		/** Why the path cannot be deployed to, if it cannot. */
		error?: string
		/** Who the next deploy makes the agent run as, as picked here. */
		onRunAsChange?: (runAs: AgentRunAs) => void
	}

	let { draft, path, workspace, error = $bindable(), onRunAsChange = undefined }: Props = $props()

	let readOnly = $derived(!draft.canWrite)
</script>

<Label label="Path">
	<Path
		bind:path={
			() => draft.state?.path,
			(v) => {
				if (draft.state && v !== undefined) draft.state.path = v
			}
		}
		bind:error
		initialPath={draft.noDeployed ? '' : path}
		checkInitialPathExistence={draft.noDeployed}
		namePlaceholder="agent"
		kind="resource"
		workspaceOverride={workspace}
		autofocus={false}
		disabled={readOnly}
	/>
</Label>
<!-- Who the deployed agent runs as when someone runs it from its page, resolved on deploy as a
     flow's on-behalf-of identity is. -->
{#if !readOnly}
	<div class="pt-2">
		<PermissionedAsLine
			permissionedAs={draft.onBehalfOf}
			path={draft.state?.path}
			onPermissionedAsChange={(permissionedAs, preserve) =>
				onRunAsChange?.({ permissionedAs, preserve })}
		/>
	</div>
{/if}
