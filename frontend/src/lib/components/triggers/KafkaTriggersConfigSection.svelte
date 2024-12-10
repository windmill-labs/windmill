<script lang="ts">
	import ResourcePicker from '../ResourcePicker.svelte'
	import { Boxes } from 'lucide-svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import SchemaForm from '../SchemaForm.svelte'

	let isValid: boolean = false

	export let defaultValues: Record<string, any> | undefined = undefined
	export let headless: boolean = false
	export let args: Record<string, any> = {}
	export let staticInputDisabled: boolean = true

	let selected: 'resource' | 'static' = staticInputDisabled ? 'resource' : 'static'

	const connnectionSchema = {
		$schema: 'http://json-schema.org/draft-07/schema#',
		type: 'object',
		properties: {
			brokers: {
				type: 'array',
				items: {
					type: 'string'
				},
				nullable: false,
				title: 'Brokers'
			},
			security: {
				type: 'object',
				title: 'Security',
				oneOf: [
					{
						type: 'object',
						title: 'PLAINTEXT',
						properties: {
							label: {
								enum: ['PLAINTEXT'],
								type: 'string'
							}
						}
					},
					{
						type: 'object',
						order: ['mechanism', 'username', 'password'],
						title: 'SASL_PLAINTEXT',
						required: ['mechanism', 'username', 'password'],
						properties: {
							label: {
								enum: ['SASL_PLAINTEXT'],
								type: 'string'
							},
							password: {
								type: 'string',
								password: true
							},
							username: {
								type: 'string'
							},
							mechanism: {
								enum: ['PLAIN', 'SCRAM-SHA-256', 'SCRAM-SHA-512'],
								type: 'string',
								disableCreate: true
							}
						}
					},
					{
						type: 'object',
						title: 'SSL',
						properties: {
							label: {
								enum: ['SSL'],
								type: 'string'
							}
						}
					},
					{
						type: 'object',
						order: ['mechanism', 'username', 'password'],
						title: 'SASL_SSL',
						required: ['mechanism', 'username', 'password'],
						properties: {
							label: {
								enum: ['SASL_SSL'],
								type: 'string'
							},
							password: {
								type: 'string',
								password: true
							},
							username: {
								type: 'string'
							},
							mechanism: {
								enum: ['PLAIN', 'SCRAM-SHA-256', 'SCRAM-SHA-512'],
								type: 'string',
								disableCreate: true
							}
						}
					}
				]
			}
		},
		required: ['brokers', 'security']
	}

	const argsSchema = {
		$schema: 'http://json-schema.org/draft-07/schema#',
		type: 'object',
		properties: {
			topics: {
				type: 'array',
				items: {
					type: 'string'
				},
				nullable: false,
				title: 'Topics'
			},
			group_id: { type: 'string', title: 'Group ID' }
		},
		required: ['topics', 'group_id']
	}

	$: connectionArgs = {
		get brokers() {
			return args.brokers
		},
		set brokers(value) {
			args.brokers = value
		},
		get security() {
			return args.security
		},
		set security(value) {
			args.security = value
		}
	}
</script>

<Section label="Kafka" {headless}>
	<div class="flex flex-col w-full gap-4">
		<div class="block grow w-full">
			<Subsection label="Connection">
				<svelte:fragment slot="header">
					{#if !staticInputDisabled}
						<ToggleButtonGroup bind:selected class="h-full">
							<ToggleButton value="static" label="Static" small={true} />
							<ToggleButton value="resource" label="Resource" icon={Boxes} small={true} />
						</ToggleButtonGroup>
					{/if}
				</svelte:fragment>

				{#if selected === 'resource'}
					<ResourcePicker
						resourceType="kafka"
						bind:value={args.kafka_resource_path}
						{defaultValues}
					/>
				{:else}
					<SchemaForm
						schema={connnectionSchema}
						bind:args={connectionArgs}
						bind:isValid
						lightHeader={true}
					/>
				{/if}
			</Subsection>
		</div>

		<div class="block grow w-full">
			<Subsection headless={true}>
				<SchemaForm schema={argsSchema} bind:args bind:isValid lightHeader={true} />
			</Subsection>
		</div>
	</div>
</Section>
