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
		isDeployed = false,
		isEditor = false,
		defaultValues = undefined,
		newDraft = false,
		customLabel = undefined,
		...props
	} = $props()

	let postgresTriggerEditor: PostgresTriggerEditorInner | undefined = $state(undefined)

	async function openPostgresTriggerEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await postgresTriggerEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path }, newDraft)
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			postgresTriggerEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		postgresTriggerEditor && openPostgresTriggerEditor(isFlow)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

<div class="flex flex-col gap-4">
	<PostgresTriggerEditorInner
		bind:this={postgresTriggerEditor}
		useDrawer={false}
		hideTarget
		hideTooltips={!isDeployed || cloudDisabled}
		allowDraft={true}
		trigger={selectedTrigger}
		{isEditor}
		{isDeployed}
		{cloudDisabled}
		{customLabel}
		{...props}
	>
		{#snippet description()}
			{#if cloudDisabled}
				<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
					Postgres triggers are disabled in the multi-tenant cloud.
				</Alert>
			{:else}
				<Description link="https://www.windmill.dev/docs/core_concepts/postgres_triggers">
					Windmill can connect to a Postgres database and trigger runnables (scripts, flows) in
					response to database transactions (INSERT, UPDATE, DELETE) on specified tables, schemas,
					or the entire database. Listening is done using Postgres's logical replication streaming
					protocol, ensuring efficient and low-latency triggering.
				</Description>
			{/if}
		{/snippet}
	</PostgresTriggerEditorInner>
</div>
