<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { PostgresTriggerService, type PostgresTrigger } from '$lib/gen'

	import { canWrite, sendUserToast } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import type { TriggerContext } from '$lib/components/triggers'
	import PostgresTriggerEditor from './PostgresTriggerEditor.svelte'
	import Description from '$lib/components/Description.svelte'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'
	import Section from '$lib/components/Section.svelte'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let isEditor: boolean = false

	let postgresTriggerEditor: PostgresTriggerEditor
	let openForm = true
	let dontCloseOnLoad = false

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'postgres' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			postgresTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let postgresTriggers: (PostgresTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			postgresTriggers = (
				await PostgresTriggerService.listPostgresTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), postgres_count: postgresTriggers?.length }
			openForm = postgresTriggers?.length === 0 || dontCloseOnLoad
		} catch (err) {
			sendUserToast(`Could not load postgres triggers ${err.body}`, true)
		}
	}
</script>

<PostgresTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={postgresTriggerEditor}
/>
{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning">
		Postgres triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description link="https://www.windmill.dev/docs/core_concepts/postgres_triggers">
			Windmill can connect to a Postgres database and trigger runnables (scripts, flows) in response
			to database transactions (INSERT, UPDATE, DELETE) on specified tables, schemas, or the entire
			database. Listening is done using Postgres's logical replication streaming protocol, ensuring
			efficient and low-latency triggering.
		</Description>

		{#if !newItem && postgresTriggers && postgresTriggers.length > 0}
			<Section label="Postgres">
				<div class="flex flex-col gap-4">
					<div class="flex flex-col divide-y pt-2">
						{#each postgresTriggers as postgresTriggers (postgresTriggers.path)}
							<div class="grid grid-cols-5 text-2xs items-center py-2">
								<div class="col-span-2 truncate">{postgresTriggers.path}</div>
								<div class="flex justify-end">
									<button
										on:click={() => postgresTriggerEditor?.openEdit(postgresTriggers.path, isFlow)}
										class="px-2"
									>
										{#if postgresTriggers.canWrite}
											Edit
										{:else}
											View
										{/if}
									</button>
								</div>
							</div>
						{/each}
					</div>
				</div>
			</Section>
		{/if}
		<TriggersEditorSection
			on:applyArgs
			on:saveTrigger={(e) => {
				postgresTriggerEditor?.openNew(isFlow, path, e.detail.config)
			}}
			on:addPreprocessor
			on:updateSchema
			on:testWithArgs
			cloudDisabled={false}
			triggerType="postgres"
			{isFlow}
			{path}
			{isEditor}
			{canHavePreprocessor}
			{hasPreprocessor}
			{newItem}
			{openForm}
		/>
	</div>
{/if}
