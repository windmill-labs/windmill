<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { NatsTriggerService, type NatsTrigger } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import NatsTriggerEditor from './NatsTriggerEditor.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Section from '$lib/components/Section.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Description from '$lib/components/Description.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false
	export let isEditor: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	let natsTriggerEditor: NatsTriggerEditor
	let openForm = true
	let dontCloseOnLoad = false

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'nats' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			natsTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let natsTriggers: (NatsTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			natsTriggers = (
				await NatsTriggerService.listNatsTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), nats_count: natsTriggers?.length }
			openForm = natsTriggers?.length === 0 || dontCloseOnLoad
		} catch (e) {
			console.error('impossible to load nats triggers', e)
		}
	}

	let data = {
		natsTriggers: [],
		newItem
	}

	function saveTrigger(path: string, args?: Record<string, any>) {
		natsTriggerEditor?.openNew(isFlow, path, args)
	}
</script>

<NatsTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={natsTriggerEditor}
/>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		Nats triggers are an enterprise only feature.
	</Alert>
{:else if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		Nats triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description link="https://www.windmill.dev/docs/core_concepts/nats_triggers">
			NATS triggers execute scripts and flows in response to messages published to NATS subjects.
		</Description>

		{#if !newItem && natsTriggers && natsTriggers.length > 0}
			<Section label="NATS Triggers">
				<div class="flex flex-col divide-y pt-2">
					{#each natsTriggers as natsTrigger (natsTrigger.path)}
						<div class="grid grid-cols-5 text-2xs items-center py-2">
							<div class="col-span-2 truncate">{natsTrigger.path}</div>
							<div class="col-span-2 truncate">
								{natsTrigger.nats_resource_path}
							</div>
							<div class="flex justify-end">
								<button
									on:click={() => natsTriggerEditor?.openEdit(natsTrigger.path, isFlow)}
									class="px-2"
								>
									{#if natsTrigger.canWrite}
										Edit
									{:else}
										View
									{/if}
								</button>
							</div>
						</div>
					{/each}
				</div>
			</Section>
		{/if}

		<TriggersEditorSection
			on:saveTrigger={(e) => {
				saveTrigger(path, e.detail.config)
			}}
			on:applyArgs
			on:addPreprocessor
			cloudDisabled={false}
			triggerType="nats"
			{isFlow}
			{data}
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
