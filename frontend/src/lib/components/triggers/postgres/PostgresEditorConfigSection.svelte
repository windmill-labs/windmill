<script lang="ts">
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Required from '$lib/components/Required.svelte'
	import MultiSelect from 'svelte-multiselect'
	import RelationPicker from './RelationPicker.svelte'
	import type { PublicationData } from '$lib/gen'
	import { emptyString } from '$lib/utils'
	import CheckPostgresRequirement from './CheckPostgresRequirement.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { X } from 'lucide-svelte'
	const DEFAULT_PUBLICATION: PublicationData = {
		transaction_to_track: ['Insert', 'Update', 'Delete'],
		table_to_track: [
			{
				schema_name: 'public',
				table_to_track: []
			}
		]
	}

	let transactionType: string[] = ['Insert', 'Update', 'Delete']
	let isValid: boolean = false

	export let headless: boolean = false
	export let can_write: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let postgres_resource_path: string = ''
	export let publication: PublicationData = DEFAULT_PUBLICATION

	function updateValidity(publication: PublicationData) {
		isValid =
			!emptyString(postgres_resource_path) &&
			(!publication.table_to_track || publication.table_to_track.length !== 0)
	}

	$: updateValidity(publication)

	let testTriggerConnection: TestTriggerConnection | undefined = undefined
</script>

<div class="h-full">
	{#if showCapture && captureInfo}
		<CaptureSection
			disabled={!isValid}
			on:captureToggle
			captureType="postgres"
			{captureInfo}
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="Postgres config" {headless}>
		<div class="flex flex-col gap-4">
			<div class="mb-2">
				<p class="text-xs mb-1 text-tertiary">
					Pick a database to connect to <Required required={true} />
				</p>
				<ResourcePicker
					disabled={!can_write}
					bind:value={postgres_resource_path}
					resourceType={'postgresql'}
					on:change={() => {
						if (emptyString(postgres_resource_path)) {
							publication = { ...DEFAULT_PUBLICATION }
						}
					}}
				/>
				{#if postgres_resource_path}
					<TestTriggerConnection
						kind="postgres"
						args={{ postgres_resource_path }}
						noButton
						bind:this={testTriggerConnection}
					/>
					<CheckPostgresRequirement
						bind:postgres_resource_path
						bind:can_write
						checkConnection={testTriggerConnection?.testTriggerConnection}
					/>
				{/if}
			</div>
			{#if postgres_resource_path}
				<Label label="Transactions">
					<svelte:fragment slot="header">
						<Tooltip small>
							Choose the types of database transactions that should trigger a script or flow. You
							can select from <strong>Insert</strong>, <strong>Update</strong>,
							<strong>Delete</strong>, or any combination of these operations to define when the
							trigger should activate.
						</Tooltip>
					</svelte:fragment>
					<MultiSelect
						noMatchingOptionsMsg=""
						createOptionMsg={null}
						duplicates={false}
						options={transactionType}
						allowUserOptions="append"
						bind:selected={publication.transaction_to_track}
						ulOptionsClass={'!bg-surface !text-sm'}
						ulSelectedClass="!text-sm"
						outerDivClass="!bg-surface !min-h-[38px] !border-[#d1d5db]"
						placeholder="Select transactions"
						--sms-options-margin="4px"
					>
						<svelte:fragment slot="remove-icon">
							<div class="hover:text-primary p-0.5">
								<X size={12} />
							</div>
						</svelte:fragment>
					</MultiSelect>
				</Label>
				<Label label="Table tracking">
					<svelte:fragment slot="header">
						<Tooltip
							documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#define-what-to-track"
							small
						>
							Select the tables to track. You can choose to track
							<strong>all tables in your database</strong>,
							<strong>all tables within a specific schema</strong>,
							<strong>specific tables in a schema</strong>, or even
							<strong>specific columns of a table</strong>. Additionally, you can apply a
							<strong>filter</strong> to retrieve only rows that do not match the specified criteria.
						</Tooltip>
					</svelte:fragment>
					<RelationPicker bind:relations={publication.table_to_track} />
				</Label>
			{/if}
		</div>
	</Section>
</div>
