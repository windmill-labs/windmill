<script lang="ts">
	import { getContext } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import type { TriggerContext } from '$lib/components/triggers'
	import PostgresTriggerEditorInner from './PostgresTriggerEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'

	let { isFlow, path, selectedTrigger, edit, isDeployed = false, isEditor = false } = $props()

	let postgresTriggerEditor: PostgresTriggerEditorInner | undefined = $state(undefined)

	const { defaultValues } = getContext<TriggerContext>('TriggerContext')

	async function openPostgresTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			postgresTriggerEditor?.openNew(isFlow, path, $defaultValues)
			if ($defaultValues) {
				defaultValues.set(undefined)
			}
		} else {
			postgresTriggerEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		postgresTriggerEditor && openPostgresTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		Postgres triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<PostgresTriggerEditorInner
			bind:this={postgresTriggerEditor}
			useDrawer={false}
			hideTarget
			editMode={edit}
			preventSave={!isDeployed}
			hideTooltips={!isDeployed}
			useEditButton
			{isEditor}
			on:toggle-edit-mode
			on:update
			on:update-config
		>
			{#snippet description()}
				<Description link="https://www.windmill.dev/docs/core_concepts/postgres_triggers">
					Windmill can connect to a Postgres database and trigger runnables (scripts, flows) in
					response to database transactions (INSERT, UPDATE, DELETE) on specified tables, schemas,
					or the entire database. Listening is done using Postgres's logical replication streaming
					protocol, ensuring efficient and low-latency triggering.
				</Description>
				{#if !isDeployed}
					<Alert
						title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the postgres trigger`}
						type="info"
						size="xs"
					/>
				{/if}
			{/snippet}
		</PostgresTriggerEditorInner>
	</div>
{/if}
