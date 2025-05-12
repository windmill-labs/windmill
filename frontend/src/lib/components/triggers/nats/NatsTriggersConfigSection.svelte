<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import { workspaceStore } from '$lib/stores'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		defaultValues?: Record<string, any> | undefined
		headless?: boolean
		natsResourcePath: string
		subjects: string[]
		useJetstream: boolean
		streamName: string
		consumerName: string
		path: string
		can_write?: boolean
		showTestingBadge?: boolean
	}

	const dispatch = createEventDispatcher()

	let {
		defaultValues = undefined,
		headless = false,
		natsResourcePath = $bindable(),
		subjects = $bindable(),
		useJetstream = $bindable(),
		streamName = $bindable(),
		consumerName = $bindable(),
		path,
		can_write = true,
		showTestingBadge = false
	}: Props = $props()

	let natsArgsCfg: Record<string, any> = $state({
		subjects,
		use_jetstream: useJetstream,
		stream_name: streamName,
		consumer_name: consumerName
	})
	let otherArgsValid = $state(false)
	let globalError = $derived(
		!useJetstream && subjects && subjects.length > 1
			? 'Only one subject is supported if not using JetStream.'
			: ''
	)
	let isConnectionValid = $derived(!!natsResourcePath)

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

	$effect(() => {
		const valid =
			isConnectionValid &&
			otherArgsValid &&
			!!subjects &&
			subjects.length > 0 &&
			subjects.every((b) => /^[a-zA-Z0-9-_.*>]+$/.test(b)) &&
			globalError === ''
		dispatch('valid-config', valid)
	})

	function setStreamAndConsumerNames() {
		if (!streamName) {
			streamName = `windmill_stream-${$workspaceStore}-${path.replaceAll('/', '__')}`
		}
		if (!consumerName) {
			consumerName = `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
		}
	}

	function updateFromArgs(args: Record<string, any>) {
		subjects = args.subjects
		useJetstream = args.use_jetstream
		streamName = args.stream_name
		consumerName = args.consumer_name
		if (args.use_jetstream) {
			setStreamAndConsumerNames()
		}
	}

	$effect(() => {
		updateFromArgs(natsArgsCfg)
	})
</script>

<div>
	<Section label="NATS" {headless}>
		{#snippet badge()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		<div class="flex flex-col w-full gap-4">
			<div class="block grow w-full">
				<Subsection label="Connection">
					<ResourcePicker
						resourceType="nats"
						bind:value={natsResourcePath}
						{defaultValues}
						disabled={!can_write}
					/>
					{#if isConnectionValid}
						<TestTriggerConnection
							kind="nats"
							args={{
								connection: {
									nats_resource_path: natsResourcePath
								}
							}}
						/>
					{/if}
				</Subsection>
			</div>

			<div class="block grow w-full">
				<Subsection headless={true}>
					<SchemaForm
						schema={argsSchema}
						bind:args={natsArgsCfg}
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
