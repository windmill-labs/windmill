<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { SqsTriggerService, type SqsTrigger } from '$lib/gen'

	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Section from '$lib/components/Section.svelte'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'
	import SqsTriggerEditor from './SqsTriggerEditor.svelte'
	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false
	export let isEditor: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	let sqsTriggerEditor: SqsTriggerEditor
	let openForm = true
	let dontCloseOnLoad = false

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'sqs' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			sqsTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let sqsTriggers: (SqsTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			sqsTriggers = (
				await SqsTriggerService.listSqsTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), sqs_count: sqsTriggers?.length }
			openForm = sqsTriggers?.length === 0 || dontCloseOnLoad
		} catch (e) {
			console.error('impossible to load SQS triggers', e)
		}
	}
</script>

<SqsTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={sqsTriggerEditor}
/>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		SQS triggers are an enterprise only feature.
	</Alert>
{:else if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		SQS triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description
			link="https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-how-it-works.html"
		>
			SQS triggers allow your scripts/flows to process messages from Amazon Simple Queue Service
			(SQS) in real time. Each trigger listens to an SQS queue and executes a script or a flow when
			new messages arrive.
		</Description>

		{#if !newItem && sqsTriggers && sqsTriggers.length > 0}
			<Section label="SQS">
				<div class="flex flex-col gap-4">
					<div class="flex flex-col divide-y pt-2">
						{#each sqsTriggers as sqsTriggers (sqsTriggers.path)}
							<div class="grid grid-cols-5 text-2xs items-center py-2">
								<div class="col-span-2 truncate">{sqsTriggers.path}</div>
								<div class="col-span-2 truncate">
									{sqsTriggers.queue_url}
								</div>
								<div class="flex justify-end">
									<button
										on:click={() => sqsTriggerEditor?.openEdit(sqsTriggers.path, isFlow)}
										class="px-2"
									>
										{#if sqsTriggers.canWrite}
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
				sqsTriggerEditor?.openNew(isFlow, path, e.detail.config)
			}}
			on:addPreprocessor
			on:updateSchema
			on:testWithArgs
			cloudDisabled={false}
			triggerType="sqs"
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
