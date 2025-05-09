<script lang="ts">
	import { Boxes } from 'lucide-svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import { workspaceStore } from '$lib/stores'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import TestingBadge from '../testingBadge.svelte'
	export let defaultValues: Record<string, any> | undefined = undefined
	export let headless: boolean = false
	export let args: Record<string, any> = {}
	export let staticInputDisabled: boolean = true
	export let isValid: boolean = false
	export let path: string
	export let can_write: boolean = true
	export let showTestingBadge: boolean = false
	let selected: 'resource' | 'static' = staticInputDisabled ? 'resource' : 'static'

	const connnectionSchema = {
		$schema: 'http://json-schema.org/draft-07/schema#',
		type: 'object',
		properties: {
			servers: {
				type: 'array',
				items: {
					type: 'string'
				},
				title: 'Servers',
				default: ['']
			},
			require_tls: {
				type: 'boolean',
				title: 'Require TLS',
				default: false
			},
			auth: {
				type: 'object',
				title: 'Authentication',
				default: {
					label: 'NO_AUTH'
				},
				oneOf: [
					{
						type: 'object',
						title: 'NO_AUTH',
						properties: {
							label: {
								enum: ['NO_AUTH'],
								type: 'string'
							}
						}
					},
					{
						type: 'object',
						order: ['token'],
						title: 'TOKEN',
						required: ['token'],
						properties: {
							label: {
								enum: ['TOKEN'],
								type: 'string'
							},
							token: {
								type: 'string',
								password: true
							}
						}
					},
					{
						type: 'object',
						title: 'USER_PASSWORD',
						required: ['user', 'password'],
						properties: {
							label: {
								enum: ['USER_PASSWORD'],
								type: 'string'
							},
							user: {
								type: 'string'
							},
							password: {
								type: 'string',
								password: true
							}
						}
					},
					{
						type: 'object',
						order: ['nkey'],
						title: 'NKEY',
						required: ['nkey'],
						properties: {
							label: {
								enum: ['NKEY'],
								type: 'string'
							},
							seed: {
								type: 'string',
								password: true
							}
						}
					},
					{
						type: 'object',
						title: 'JWT',
						required: ['jwt', 'seed'],
						properties: {
							label: {
								enum: ['JWT'],
								type: 'string'
							},
							jwt: {
								type: 'string',
								password: true
							},
							seed: {
								type: 'string',
								password: true
							}
						}
					}
				]
			}
		},
		required: ['servers', 'auth', 'require_tls']
	}

	const argsSchema = {
		$schema: 'http://json-schema.org/draft-07/schema#',
		type: 'object',
		properties: {
			subjects: {
				type: 'array',
				items: {
					type: 'string'
				},
				nullable: false,
				title: 'Subjects',
				default: [''],
				description:
					'The subjects to listen to. If not using JetStream, only one subject is supported.'
			},
			use_jetstream: {
				type: 'boolean',
				title: 'Use JetStream',
				default: false,
				description: 'Whether to use JetStream (durable push-consumer)'
			},
			stream_name: {
				type: 'string',
				title: 'Stream name',
				pattern: '^((\\$var:[\\w-\\/]+)|[a-zA-Z0-9-_]+)$',
				customErrorMessage: 'Invalid stream name',
				showExpr: 'fields.use_jetstream',
				description:
					'Required if using JetStream. If the stream already exists, it will be updated to include the specified subjects if not already present. Otherwise, it will be created.'
			},
			consumer_name: {
				type: 'string',
				title: 'Consumer name',
				pattern: '^((\\$var:[\\w-\\/]+)|[a-zA-Z0-9-_]+)$',
				customErrorMessage: 'Invalid consumer name',
				showExpr: 'fields.use_jetstream',
				description:
					'Required is using JetStream. If a consumer with the same name already exists, it will be overwritten. It is also used as the deliver subject.'
			}
		},
		required: ['subjects', 'use_jetstream', 'stream_name', 'consumer_name']
	}

	let isStaticConnectionValid = false
	let otherArgsValid = false
	let globalError = ''

	$: globalError =
		!args.use_jetstream && args.subjects && args.subjects.length > 1
			? 'Only one subject is supported if not using JetStream.'
			: ''

	$: isConnectionValid =
		selected === 'resource'
			? !!args.nats_resource_path
			: isStaticConnectionValid &&
				args.servers &&
				args.servers.length > 0 &&
				args.servers.every((b) => b.length > 0) &&
				args.require_tls !== undefined &&
				args.require_tls !== null

	$: isValid =
		isConnectionValid &&
		otherArgsValid &&
		args.subjects &&
		args.subjects.length > 0 &&
		args.subjects.every((b) => /^[a-zA-Z0-9-_.*>]+$/.test(b)) &&
		globalError === ''

	$: usingResource = !!args.nats_resource_path
	$: usingResource && (selected = 'resource')

	function setStreamAndConsumerNames() {
		if (!args.stream_name) {
			args.stream_name = `windmill_stream-${$workspaceStore}-${path.replaceAll('/', '__')}`
		}
		if (!args.consumer_name) {
			args.consumer_name = `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
		}
	}
	$: useJetStream = args.use_jetstream
	$: useJetStream && setStreamAndConsumerNames()
</script>

<div>
	<Section label="NATS" {headless}>
		<svelte:fragment slot="badge">
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
								disabled={!can_write}
								on:selected={(ev) => {
									if (ev.detail === 'static') {
										delete args.nats_resource_path
										args.require_tls = false
										args.servers = ['']
										args.auth = {
											label: 'NO_AUTH'
										}
									} else {
										delete args.servers
										delete args.auth
										delete args.require_tls
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
							resourceType="nats"
							bind:value={args.nats_resource_path}
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
						<TestTriggerConnection kind="nats" args={{ connection: args }} />
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
				<span class="text-xs text-red-500">{globalError}</span>
			</div>
		</div>
	</Section>
</div>
