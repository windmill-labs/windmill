<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { GcpTriggerService, type GcpTrigger } from '$lib/gen'

	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Section from '$lib/components/Section.svelte'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'
	import GcpTriggerEditor from './GcpTriggerEditor.svelte'
	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false
	export let isEditor: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	let gcpTriggerEditor: GcpTriggerEditor
	let openForm = true
	let dontCloseOnLoad = false

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'gcp' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			gcpTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let gcpTriggers: (GcpTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			gcpTriggers = (
				await GcpTriggerService.listGcpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), gcp_count: gcpTriggers?.length }
			openForm = gcpTriggers?.length === 0 || dontCloseOnLoad
		} catch (e) {
			console.error('impossible to load GCP Pub/Sub triggers', e)
		}
	}
</script>

<GcpTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={gcpTriggerEditor}
/>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		GCP Pub/Sub triggers are an enterprise only feature.
	</Alert>
{:else if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		GCP Pub/Sub triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description link="https://cloud.google.com/pubsub/docs?hl=en">
			GCP Pub/Sub triggers allow your scripts or flows to process messages from Google Cloud
			Pub/Sub in real time. Each trigger listens to a Pub/Sub subscription and executes a script or
			a flow when new messages are published to the corresponding topic.
		</Description>

		{#if !newItem && gcpTriggers && gcpTriggers.length > 0}
			<Section label="GCP Pub/Sub">
				<div class="flex flex-col gap-4">
					<div class="flex flex-col divide-y pt-2">
						{#each gcpTriggers as gcpTrigger (gcpTrigger.path)}
							<div class="grid grid-cols-5 text-2xs items-center py-2">
								<div class="col-span-2 truncate">{gcpTrigger.path}</div>

								<div class="flex justify-end">
									<button
										on:click={() => gcpTriggerEditor?.openEdit(gcpTrigger.path, isFlow)}
										class="px-2"
									>
										{#if gcpTrigger.canWrite}
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
				gcpTriggerEditor?.openNew(isFlow, path, e.detail.config)
			}}
			on:addPreprocessor
			on:updateSchema
			on:testWithArgs
			cloudDisabled={false}
			triggerType="gcp"
			{isFlow}
			{path}
			{isEditor}
			{canHavePreprocessor}
			{hasPreprocessor}
			{newItem}
			{openForm}
			bind:showCapture={dontCloseOnLoad}
		/>
	</div>
{/if}
