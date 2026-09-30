<script lang="ts">
	import { resource } from 'runed'
	import Label from '$lib/components/Label.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import ResourceDescriptionField from '$lib/components/ResourceDescriptionField.svelte'
	import ResourcePathHint from '$lib/components/ResourcePathHint.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import PermissionedAsLine from '$lib/components/triggers/PermissionedAsLine.svelte'
	import { WorkspaceService } from '$lib/gen'
	import type { AgentDraftHandle, AgentRunAs } from '../agentDraft.svelte'

	/** An agent's own settings, laid out as the top of the resource editor: what it is saved as
	 *  rather than what it does. */
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
	// Only a workspace that deploys somewhere has anything to keep an agent out of.
	const deployTo = resource(
		() => workspace,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)
</script>

{#if draft.state}
	<div class="flex flex-col gap-6">
		<Label label="Path">
			<ResourcePathHint />
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
		<!-- Who the deployed agent runs as when someone runs it from its page, resolved on deploy as
		     a flow's on-behalf-of identity is. -->
		{#if !readOnly}
			<PermissionedAsLine
				permissionedAs={draft.onBehalfOf}
				path={draft.state.path}
				onPermissionedAsChange={(permissionedAs, preserve) =>
					onRunAsChange?.({ permissionedAs, preserve })}
			/>
		{/if}
		<LabelsInput bind:labels={draft.state.labels} {workspace} class="-mt-4" />
		{#if deployTo.current}
			<Label label="Workspace specific" tooltip="Keeps this agent out of deploys to prod/staging.">
				<Toggle bind:checked={draft.state.wsSpecific} disabled={readOnly} />
			</Label>
		{/if}
		<ResourceDescriptionField
			bind:description={draft.state.description}
			label="Description"
			placeholder="Describe what this agent does"
			canWrite={!readOnly}
		/>
	</div>
{/if}
