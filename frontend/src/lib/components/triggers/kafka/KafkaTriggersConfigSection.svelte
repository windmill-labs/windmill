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
	import { workspaceStore } from '$lib/stores'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import TestingBadge from '../testingBadge.svelte'

	export let path: string
	export let defaultValues: Record<string, any> | undefined = undefined
	export let headless: boolean = false
	export let args: Record<string, any> = {}
	export let staticInputDisabled: boolean = true
	export let showCapture: boolean = false
	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined
	export let isValid: boolean = false
	export let can_write: boolean = true
	export let showTestingBadge: boolean = false

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
				title: 'Brokers',
				default: ['']
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
						order: ['ca', 'certificate', 'key', 'key_password'],
						properties: {
							label: {
								enum: ['SSL'],
								type: 'string'
							},
							ca: {
								type: 'string',
								description: 'CA certificate to verify the server certificate, in PEM format'
							},
							certificate: {
								type: 'string',
								description: 'Client certificate for authentication to the server, in PEM format'
							},
							key: {
								type: 'string',
								description: 'Client key for authentication to the server, in PEM format'
							},
							key_password: {
								type: 'string',
								description: 'Password to decrypt the client key, if encrypted',
								password: true
							}
						}
					},
					{
						type: 'object',
						order: [
							'mechanism',
							'username',
							'password',
							'ca',
							'certificate',
							'key',
							'key_password'
						],
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
							},
							ca: {
								type: 'string',
								description: 'CA certificate to verify the server certificate, in PEM format'
							},
							certificate: {
								type: 'string',
								description: 'Client certificate for authentication to the server, in PEM format'
							},
							key: {
								type: 'string',
								description: 'Client key for authentication to the server, in PEM format'
							},
							key_password: {
								type: 'string',
								description: 'Password to decrypt the client key, if encrypted',
								password: true
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

	let isStaticConnectionValid = false
	let otherArgsValid = false

	$: isConnectionValid =
		selected === 'resource'
			? !!args.kafka_resource_path
			: isStaticConnectionValid &&
				args.brokers &&
				args.brokers.length > 0 &&
				args.brokers.every((b) => b.length > 0)

	$: isValid =
		isConnectionValid &&
		otherArgsValid &&
		args.topics &&
		args.topics.length > 0 &&
		args.topics.every((b) => /^[a-zA-Z0-9-_.]+$/.test(b))

	$: usingResource = !!args.kafka_resource_path
	$: usingResource && (selected = 'resource')

	function setGroupId() {
		if (!args.group_id) {
			args.group_id = `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
		}
	}
	$: path && setGroupId()
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
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="Kafka" {headless}>
		<svelte:fragment slot="header">
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		</svelte:fragment>
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
								let:item
							>
								<ToggleButton
									value="static"
									label="Static"
									small={true}
									{item}
									disabled={!can_write}
								/>
								<ToggleButton
									value="resource"
									label="Resource"
									icon={Boxes}
									small={true}
									{item}
									disabled={!can_write}
								/>
							</ToggleButtonGroup>
						{/if}
					</svelte:fragment>

					{#if selected === 'resource'}
						<ResourcePicker
							resourceType="kafka"
							bind:value={args.kafka_resource_path}
							{defaultValues}
							disabled={!can_write}
						/>
					{:else}
						<SchemaForm
							schema={connnectionSchema}
							bind:args
							bind:isValid={isStaticConnectionValid}
							lightHeader={true}
							disabled={!can_write}
						/>
					{/if}
					{#if isConnectionValid}
						<TestTriggerConnection
							kind="kafka"
							args={{
								connection: args
							}}
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
						disabled={!can_write}
					/>
				</Subsection>
			</div>
		</div>
	</Section>
</div>
