<script lang="ts">
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Required from '$lib/components/Required.svelte'
	import MultiSelect from 'svelte-multiselect'
	import RelationPicker from './RelationPicker.svelte'
	import type { PublicationData } from '$lib/gen'
	import { emptyString } from '$lib/utils'
	import CheckPostgresRequirement from './CheckPostgresRequirement.svelte'

	let transactionType: string[] = ['Insert', 'Update', 'Delete']
	export let headless: boolean = false
	export let can_write: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let postgres_resource_path: string = ''
	export let publication: PublicationData = {
		transaction_to_track: ['Insert', 'Update', 'Delete'],
		table_to_track: [
			{
				schema_name: 'public',
				table_to_track: []
			}
		]
	}
	$: publication =
		publication === undefined
			? {
					transaction_to_track: ['Insert', 'Update', 'Delete'],
					table_to_track: [
						{
							schema_name: 'public',
							table_to_track: []
						}
					]
			  }
			: publication
	let notEmpty = publication.table_to_track && publication.table_to_track.length > 0

	$: notEmpty = publication.table_to_track && publication.table_to_track.length > 0
	let selectedTable: 'all' | 'specific' = notEmpty ? 'specific' : 'all'
	$: isValid =
		!emptyString(postgres_resource_path) &&
		publication.transaction_to_track.length > 0 &&
		(selectedTable === 'all' || (notEmpty ?? false))
	$: selectedTable === 'all' && (publication.table_to_track = [])
</script>

<div>
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
		<div class="flex flex-col gap-3">
			<div class="mb-2">
				<p class="text-xs mb-1 text-tertiary">
					Pick a database to connect to <Required required={true} />
				</p>
				<ResourcePicker
					disabled={!can_write}
					bind:value={postgres_resource_path}
					resourceType={'postgresql'}
				/>
				{#if postgres_resource_path}
					<TestTriggerConnection kind="postgres" args={{ postgres_resource_path }} />
					<CheckPostgresRequirement bind:postgres_resource_path bind:can_write />
				{/if}
			</div>
			{#if postgres_resource_path}
				<Section label="Transactions">
					<p class="text-xs mb-3 text-tertiary">
						Choose the types of database transactions that should trigger a script or flow. You can
						select from <strong>Insert</strong>, <strong>Update</strong>,
						<strong>Delete</strong>, or any combination of these operations to define when the
						trigger should activate.
					</p>
					<MultiSelect
						ulOptionsClass={'!bg-surface-secondary'}
						noMatchingOptionsMsg=""
						createOptionMsg={null}
						duplicates={false}
						options={transactionType}
						allowUserOptions="append"
						bind:selected={publication.transaction_to_track}
					/>
				</Section>
				<Section label="Table tracking">
					<p class="text-xs mb-3 text-tertiary">
						Select the tables to track. You can choose to track
						<strong>all tables in your database</strong>,
						<strong>all tables within a specific schema</strong>,
						<strong>specific tables in a schema</strong>, or even
						<strong>specific columns of a table</strong>. Additionally, you can apply a
						<strong>filter</strong> to retrieve only rows that do not match the specified criteria.
					</p>
					<RelationPicker bind:selectedTable bind:relations={publication.table_to_track} />
				</Section>
			{/if}
		</div>
	</Section>
</div>
