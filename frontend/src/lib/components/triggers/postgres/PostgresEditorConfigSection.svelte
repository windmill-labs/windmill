<script lang="ts">
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Required from '$lib/components/Required.svelte'
	import MultiSelect from 'svelte-multiselect'
	import RelationPicker from './RelationPicker.svelte'
	import type { Relations } from '$lib/gen'
	import { emptyString } from '$lib/utils'

	let transactionType: string[] = ['Insert', 'Update', 'Delete']
	let selectedTable: 'all' | 'specific' = 'specific'
	export let transaction_to_track: string[] = ['Insert', 'Update', 'Delete']
	export let relations: Relations[] = []
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let postgres_resource_path: string = ''
	$: isValid =
		!emptyString(postgres_resource_path) &&
		transaction_to_track?.length > 0 &&
		(selectedTable === 'all' || table_to_track?.length > 0)
	$: table_to_track = selectedTable === 'all' ? [] : relations
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
				<ResourcePicker bind:value={postgres_resource_path} resourceType={'postgresql'} />
				{#if postgres_resource_path}
					<TestTriggerConnection kind="postgres" args={{ postgres_resource_path }} />
				{/if}
			</div>
			<Section label="Transactions">
				<p class="text-xs mb-3 text-tertiary">
					Choose the types of database transactions that should trigger a script or flow. You can
					select from <strong>Insert</strong>, <strong>Update</strong>,
					<strong>Delete</strong>, or any combination of these operations to define when the trigger
					should activate.
				</p>
				<MultiSelect
					ulOptionsClass={'!bg-surface-secondary'}
					noMatchingOptionsMsg=""
					createOptionMsg={null}
					duplicates={false}
					bind:value={transaction_to_track}
					options={transactionType}
					allowUserOptions="append"
					bind:selected={transaction_to_track}
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
				<RelationPicker bind:selectedTable bind:relations />
			</Section>
		</div>
	</Section>
</div>
