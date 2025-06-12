<script lang="ts">
	import ResourcePicker from '../../ResourcePicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import SchemaForm from '../../SchemaForm.svelte'
	import { workspaceStore } from '$lib/stores'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import { untrack } from 'svelte'

	interface Props {
		path: string
		kafkaCfgValid?: boolean
		kafkaResourcePath?: string
		kafkaCfg?: Record<string, any>
		can_write?: boolean
		showTestingBadge?: boolean
	}

	let {
		path,
		kafkaCfgValid = $bindable(false),
		kafkaResourcePath = $bindable(''),
		kafkaCfg = $bindable({}),
		can_write = true,
		showTestingBadge = false
	}: Props = $props()

	const kafkaConfigSchema = {
		$schema: 'http://json-schema.org/draft-07/schema#',
		type: 'object',
		properties: {
			topics: {
				type: 'array',
				items: {
					type: 'string'
				},
				nullable: false,
				title: 'Topics',
				default: ['']
			},
			group_id: {
				type: 'string',
				title: 'Group ID',
				pattern: '^((\\$var:[\\w-\\/]+)|[a-zA-Z0-9-_.]+)$',
				customErrorMessage: 'Invalid group ID'
			}
		},
		required: ['topics', 'group_id']
	}

	function setGroupId() {
		if (!kafkaCfg.group_id) {
			kafkaCfg.group_id = `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
		}
	}

	$effect(() => {
		path && untrack(() => setGroupId())
	})
</script>

<div>
	<Section label="Kafka">
		{#snippet header()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		<div class="flex flex-col w-full gap-4">
			<div class="block grow w-full">
				<Subsection label="Connection">
					<ResourcePicker
						resourceType="kafka"
						bind:value={kafkaResourcePath}
						disabled={!can_write}
					/>
					{#if !!kafkaResourcePath}
						<TestTriggerConnection
							kind="kafka"
							args={{
								connection: {
									kafka_resource_path: kafkaResourcePath
								}
							}}
						/>
					{/if}
				</Subsection>
			</div>

			<div class="block grow w-full">
				<Subsection headless={true}>
					<SchemaForm
						schema={kafkaConfigSchema}
						bind:args={kafkaCfg}
						bind:isValid={kafkaCfgValid}
						lightHeader={true}
						disabled={!can_write}
					/>
				</Subsection>
			</div>
		</div>
	</Section>
</div>
