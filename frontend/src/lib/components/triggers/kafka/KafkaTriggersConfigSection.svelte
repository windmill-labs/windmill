<script lang="ts">
	import ResourcePicker from '../../ResourcePicker.svelte'
	import { Boxes } from 'lucide-svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ToggleButton from '../../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import SchemaForm from '../../SchemaForm.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	export let defaultValues: Record<string, any> | undefined = undefined
	export let headless: boolean = false
	export let args: Record<string, any> = {}
	export let staticInputDisabled: boolean = true
	export let showCapture: boolean = false
	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined
	export let isValid: boolean = false

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
			group_id: {
				type: 'string',
				title: 'Group ID',
				pattern: '^[a-zA-Z0-9-_.]+$',
				customErrorMessage: 'Invalid group ID'
			}
		},
		required: ['topics', 'group_id']
	}

	let connectionValid = false
	let otherArgsValid = false

	$: isValid =
		(selected === 'resource'
			? !!args.kafka_resource_path
			: connectionValid &&
			  args.brokers &&
			  args.brokers.length > 0 &&
			  args.brokers.every((b) => b.length > 0)) &&
		otherArgsValid &&
		args.topics &&
		args.topics.length > 0 &&
		args.topics.every((b) => /^[a-zA-Z0-9-_.]+$/.test(b))

	$: args.kafka_resource_path && (selected = 'resource')
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			captureType="kafka"
			disabled={!isValid}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			bind:captureTable
		/>
	{/if}
	<Section label="Kafka" {headless}>
		<div class="flex flex-col w-full gap-4">
			<div class="block grow w-full">
				<Subsection label="Connection">
					<svelte:fragment slot="header">
						{#if !staticInputDisabled}
							<ToggleButtonGroup
								bind:selected
								class="h-full"
								on:selected={(ev) => {
									if (ev.detail === 'static') {
										delete args.kafka_resource_path
										args.brokers = ['']
										args.security = {
											label: 'PLAINTEXT'
										}
									} else {
										delete args.brokers
										delete args.security
									}
								}}
							>
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
							bind:args
							bind:isValid={connectionValid}
							lightHeader={true}
						/>
					{/if}
				</Subsection>
			</div>

			<div class="block grow w-full">
				<Subsection headless={true}>
					<SchemaForm
						schema={argsSchema}
						bind:args
						bind:isValid={otherArgsValid}
						lightHeader={true}
					/>
				</Subsection>
			</div>
		</div>
	</Section>
</div>
