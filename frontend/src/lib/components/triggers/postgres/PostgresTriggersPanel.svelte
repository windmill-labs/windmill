<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import PostgresTriggerEditorInner from './PostgresTriggerEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { onMount } from 'svelte'

	let {
		isFlow,
		path,
		selectedTrigger,
		edit,
		isDeployed = false,
		isEditor = false,
		defaultValues = undefined,
		newDraft = false
	} = $props()

	let postgresTriggerEditor: PostgresTriggerEditorInner | undefined = $state(undefined)

	async function openPostgresTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			postgresTriggerEditor?.openNew(isFlow, path, defaultValues, newDraft)
		} else {
			postgresTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
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
			hideTooltips={!isDeployed}
			allowDraft={true}
			hasDraft={!!selectedTrigger.draftConfig}
			isDraftOnly={selectedTrigger.isDraft}
			{isEditor}
			on:toggle-edit-mode
			on:update
			on:update-config
			on:delete
			on:save-draft
			on:reset
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
