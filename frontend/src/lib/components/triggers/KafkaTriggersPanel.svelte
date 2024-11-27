<script lang="ts">
	import { Button } from '../common'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { KafkaTriggerService, type KafkaTrigger } from '$lib/gen'
	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import { canWrite } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import type { TriggerContext } from '../triggers'
	import { getContext, onMount } from 'svelte'
	import KafkaTriggerEditor from './KafkaTriggerEditor.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import KafkaIcon from '../icons/KafkaIcon.svelte'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false

	let kafkaTriggerEditor: KafkaTriggerEditor

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'kafka' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			kafkaTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let kafkaTriggers: (KafkaTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			kafkaTriggers = (
				await KafkaTriggerService.listKafkaTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), kafka_count: kafkaTriggers?.length }
		} catch (e) {
			console.error('impossible to load kafka triggers', e)
		}
	}
</script>

<KafkaTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={kafkaTriggerEditor}
/>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		Kafka triggers are an enterprise only feature.
	</Alert>
{:else if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		Kafka triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		{#if newItem}
			<Alert title="Triggers disabled" type="warning" size="xs">
				Deploy the {isFlow ? 'flow' : 'script'} to add kafka triggers.
			</Alert>
		{:else}
			<Button
				on:click={() => kafkaTriggerEditor?.openNew(isFlow, path)}
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: KafkaIcon }}
			>
				New Kafka Trigger
			</Button>
			{#if kafkaTriggers}
				{#if kafkaTriggers.length == 0}
					<div class="text-xs text-secondary"> No kafka triggers </div>
				{:else}
					<div class="flex flex-col divide-y pt-2">
						{#each kafkaTriggers as kafkaTrigger (kafkaTrigger.path)}
							<div class="grid grid-cols-5 text-2xs items-center py-2">
								<div class="col-span-2 truncate">{kafkaTrigger.path}</div>
								<div class="col-span-2 truncate">
									{kafkaTrigger.kafka_resource_path}
								</div>
								<div class="flex justify-end">
									<button
										on:click={() => kafkaTriggerEditor?.openEdit(kafkaTrigger.path, isFlow)}
										class="px-2"
									>
										{#if kafkaTrigger.canWrite}
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
		{/if}
	</div>
{/if}
