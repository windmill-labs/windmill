<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { PostgresTriggerService, type PostgresTrigger } from '$lib/gen'

	import { canWrite, sendUserToast } from '$lib/utils'
	import { getContext } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert, Skeleton } from '$lib/components/common'
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

	$: path && loadTriggers()

	const { triggersCount } = getContext<TriggerContext>('TriggerContext')

	let databaseTriggers: (PostgresTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			databaseTriggers = (
				await PostgresTriggerService.listPostgresTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), postgres_count: databaseTriggers?.length }
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
		/>
		{#if !newItem}
			<Section label="Postgres">
				<div class="flex flex-col gap-4">
					{#if databaseTriggers}
						{#if databaseTriggers.length == 0}
							<div class="text-xs text-secondary"> No Postgres triggers </div>
						{:else}
							<div class="flex flex-col divide-y pt-2">
								{#each databaseTriggers as databaseTriggers (databaseTriggers.path)}
									<div class="grid grid-cols-5 text-2xs items-center py-2">
										<div class="col-span-2 truncate">{databaseTriggers.path}</div>
										<div class="flex justify-end">
											<button
												on:click={() =>
													postgresTriggerEditor?.openEdit(databaseTriggers.path, isFlow)}
												class="px-2"
											>
												{#if databaseTriggers.canWrite}
													Edit
												{:else}
													View
												{/if}
											</button>
										</div>
									</div>
								{/each}
							</div>
						{/if}
					{:else}
						<Skeleton layout={[[8]]} />
					{/if}
				</div>
			</Section>
		{/if}
	</div>
{/if}
