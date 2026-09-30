<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import type { AgentDraftHandle } from '../agentDraft.svelte'

	interface Props {
		draft: AgentDraftHandle
		/** The path the editor opened, which a first deploy of a new agent replaces. */
		path: string
		workspace: string | undefined
		/** Why the path cannot be deployed to, if it cannot. */
		error?: string
	}

	let { draft, path, workspace, error = $bindable() }: Props = $props()

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
<!-- An agent runs as whoever runs it, so sharing one through a folder shares nothing it uses. -->
{#if !readOnly && draft.state?.path?.startsWith('f/')}
	<Alert type="info" size="xs" title="Shared through its folder" class="mt-2">
		Anyone who runs this agent needs access to its AI resource, and to the resources and workspace
		scripts its tools use.
	</Alert>
{/if}
