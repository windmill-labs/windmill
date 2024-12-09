<script lang="ts">
	import { fade } from 'svelte/transition'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { X, Plus, Boxes } from 'lucide-svelte'
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from '$lib/components/Label.svelte'
	import SchemaForm from '../SchemaForm.svelte'

	let isValid: boolean = false

	export let kafka_resource_path: string = ''
	export let topics: string[] = ['']
	export let group_id: string = ''
	export let topicsError: string = ''
	export let groupIdError: string = ''
	export let defaultValues: Record<string, any> | undefined = undefined
	export let dirtyGroupId: boolean = false
	export let headless: boolean = false
	export let args: Record<string, any> | undefined = undefined
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

	console.log(argsSchema)

	function updateArgs(topics: string[], group_id: string) {
		if (args) {
			args.topics = topics
			args.group_id = group_id
		}
	}

	$: updateArgs(topics, group_id)
</script>

<Section label="Kafka" {headless}>
	<div class="flex flex-col w-full gap-4">
		<div class="block grow w-full">
			<Label label="Connection">
				<svelte:fragment slot="header">
					{#if !staticInputDisabled}
						<ToggleButtonGroup bind:selected>
							<ToggleButton value="static" label="Static" small />
							<ToggleButton value="resource" label="Resource" icon={Boxes} small />
						</ToggleButtonGroup>
					{/if}
				</svelte:fragment>

				{#if selected === 'resource'}
					<ResourcePicker resourceType="kafka" bind:value={kafka_resource_path} {defaultValues} />
				{:else}
					<SchemaForm schema={connnectionSchema} bind:args bind:isValid />
				{/if}
			</Label>
		</div>
		<label class="block grow w-full">
			<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
				<div>
					Topics
					<Required required={true} />
				</div>
			</div>

			<div class="flex flex-col gap-4 mt-1">
				{#each topics as v, i}
					<div class="flex w-full gap-2 items-center">
						<input type="text" bind:value={v} />

						<button
							transition:fade|local={{ duration: 100 }}
							class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
							aria-label="Clear"
							on:click={() => {
								topics = topics.filter((_, index) => index !== i)
							}}
							class:hidden={topics.length === 1}
						>
							<X size={14} />
						</button>
					</div>
				{/each}

				<div class="flex items-baseline">
					<Button
						variant="border"
						color="light"
						size="xs"
						on:click={() => {
							if (topics == undefined || !Array.isArray(topics)) {
								topics = []
							}
							topics = topics.concat('')
						}}
						startIcon={{ icon: Plus }}
					>
						Add topic
					</Button>
				</div>
			</div>
			<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
				{topicsError}
			</div>
		</label>

		<label class="block grow w-full">
			<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
				<div>
					Group ID
					<Required required={true} />
				</div>
			</div>
			<div class="mt-1">
				<input type="text" bind:value={group_id} on:focus={() => (dirtyGroupId = true)} />
			</div>

			<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
				{groupIdError}
			</div>
		</label>
	</div>
</Section>
