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
	import { PostgresTriggerService, type Relations } from '$lib/gen'
	import { emptyString, emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import CheckPostgresRequirement from './CheckPostgresRequirement.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Loader2, X } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { Button, Tab, TabContent, Tabs } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import SlotPicker from './SlotPicker.svelte'
	import PublicationPicker from './PublicationPicker.svelte'
	import { random_adj } from '$lib/components/random_positive_adjetive'

	let transactionType: string[] = ['Insert', 'Update', 'Delete']
	let isValid: boolean = false
	let publicationItems: string[] = []

	let loadingPostgres = false
	let postgresVersion = ''
	let tab: 'advanced' | 'basic' = 'basic'

	type actions = 'create' | 'get'
	let selectedPublicationAction: actions = 'create'
	let selectedSlotAction: actions = 'create'

	export let edit: boolean = false
	export let transaction_to_track: string[] = ['Insert', 'Update', 'Delete']
	export let relations: Relations[]
	export let postgres_resource_path: string = ''
	export let publication_name: string = ''
	export let replication_slot_name: string = ''
	export let headless: boolean = false
	export let can_write: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined

	async function createPublication() {
		try {
			const message = await PostgresTriggerService.createPostgresPublication({
				path: postgres_resource_path,
				publication: publication_name as string,
				workspace: $workspaceStore!,
				requestBody: {
					transaction_to_track: transaction_to_track,
					table_to_track: relations
				}
			})

			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	async function createSlot() {
		try {
			const message = await PostgresTriggerService.createPostgresReplicationSlot({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				requestBody: {
					name: replication_slot_name
				}
			})
			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	$: {
		if (postgres_resource_path) {
			loadingPostgres = true
			PostgresTriggerService.getPostgresVersion({
				workspace: $workspaceStore!,
				path: postgres_resource_path
			})
				.then((version: string) => {
					postgresVersion = version
				})
				.catch((error: any) => {
					sendUserToast(error.body, true)
				})
				.finally(() => {
					loadingPostgres = false
				})
		}
	}

	$: {
		isValid = !emptyString(postgres_resource_path) && transaction_to_track.length > 0
	}

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
	<Section label="Database" {headless}>
		<p class="text-xs text-tertiary mb-2">
			Pick a database to connect to <Required required={true} />
		</p>
		<div class="flex flex-col gap-8">
			<div class="flex flex-col gap-2">
				<ResourcePicker
					disabled={!can_write}
					bind:value={postgres_resource_path}
					resourceType={'postgresql'}
				/>
				<CheckPostgresRequirement
					checkConnection={testTriggerConnection}
					bind:postgres_resource_path
					bind:can_write
				/>
			</div>

			{#if loadingPostgres}
				<div class="flex flex-col items-center justify-center h-full w-full">
					<Loader2 size="50" class="animate-spin" />
					<p>Loading...</p>
				</div>
			{:else if postgres_resource_path}
				<Label label="Transactions">
					<svelte:fragment slot="header">
						<Tooltip>
							<p>
								Choose the types of database transactions that should trigger a script or flow. You
								can select from <strong>Insert</strong>, <strong>Update</strong>,
								<strong>Delete</strong>, or any combination of these operations to define when the
								trigger should activate.
							</p>
						</Tooltip>
					</svelte:fragment>
					<MultiSelect
						noMatchingOptionsMsg=""
						createOptionMsg={null}
						duplicates={false}
						options={transactionType}
						allowUserOptions="append"
						bind:selected={transaction_to_track}
						ulOptionsClass={'!bg-surface !text-sm'}
						ulSelectedClass="!text-sm"
						outerDivClass="!bg-surface !min-h-[38px] !border-[#d1d5db]"
						placeholder="Select transactions"
						--sms-options-margin="4px"
						--sms-open-z-index="100"
					>
						<svelte:fragment slot="remove-icon">
							<div class="hover:text-primary p-0.5">
								<X size={12} />
							</div>
						</svelte:fragment>
					</MultiSelect>
				</Label>
				<Label label="Table Tracking">
					<svelte:fragment slot="header">
						<Tooltip
							documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#define-what-to-track"
						>
							<p>
								Select the tables to track. You can choose to track
								<strong>all tables in your database</strong>,
								<strong>all tables within a specific schema</strong>,
								<strong>specific tables in a schema</strong>, or even
								<strong>specific columns of a table</strong>. Additionally, you can apply a
								<strong>filter</strong> to retrieve only rows that do not match the specified criteria.
							</p>
						</Tooltip>
					</svelte:fragment>
					<Tabs bind:selected={tab}>
						<Tab value="basic"
							><div class="flex flex-row gap-1"
								>Basic<Tooltip
									documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#define-what-to-track"
									><p
										>Choose the <strong>relations</strong> to track without worrying about the
										underlying mechanics of creating a
										<strong>publication</strong>
										or <strong>slot</strong>. This simplified option lets you focus only on the data
										you want to monitor.</p
									></Tooltip
								></div
							></Tab
						>
						<Tab value="advanced"
							><div class="flex flex-row gap-1"
								>Advanced<Tooltip
									documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#advanced"
									><p
										>Select a specific <strong>publication</strong> from your database to track, and
										manage it by <strong>creating</strong>,
										<strong>updating</strong>, or <strong>deleting</strong>. For
										<strong>slots</strong>, you can <strong>create</strong> or
										<strong>delete</strong>
										them. Both <strong>non-active slots</strong> and the
										<strong>currently used slot</strong> by the trigger will be retrieved from your database
										for management.</p
									></Tooltip
								></div
							></Tab
						>
						<svelte:fragment slot="content">
							<div class="mt-5 overflow-hidden bg-surface">
								<TabContent value="basic">
									<RelationPicker {can_write} bind:relations {postgresVersion} />
								</TabContent>
								<TabContent value="advanced">
									<div class="flex flex-col gap-6"
										><Section
											small
											label="Replication slot"
											tooltip="Choose and manage the slots for your trigger. You can create or delete slots. Both non-active slots and the currently used slot by the trigger (if any) will be retrieved from your database for management."
											documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#managing-postgres-replication-slots"
										>
											<div class="flex flex-col gap-3">
												<ToggleButtonGroup
													bind:selected={selectedSlotAction}
													on:selected={() => {
														replication_slot_name = ''
													}}
													disabled={!can_write}
													let:item
												>
													<ToggleButton value="create" label="Create Slot" {item} />
													<ToggleButton value="get" label="Get Slot" {item} />
												</ToggleButtonGroup>
												{#if selectedSlotAction === 'create'}
													<div class="flex gap-3">
														<input
															type="text"
															bind:value={replication_slot_name}
															placeholder={'Choose a slot name'}
														/>
														<Button
															color="light"
															size="xs"
															variant="border"
															disabled={emptyStringTrimmed(replication_slot_name)}
															on:click={createSlot}>Create</Button
														>
													</div>
												{:else}
													<SlotPicker {edit} {postgres_resource_path} bind:replication_slot_name />
												{/if}
											</div>
										</Section>

										<Section
											small
											label="Publication"
											tooltip="Select and manage the publications for tracking data. You can create, update, or delete publications. Only existing publications in your database will be available for selection, giving you full control over what data is tracked."
											documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#managing-postgres-publications"
										>
											<div class="flex flex-col gap-3">
												<ToggleButtonGroup
													selected={selectedPublicationAction}
													disabled={!can_write}
													on:selected={({ detail }) => {
														selectedPublicationAction = detail
														if (selectedPublicationAction === 'create') {
															publication_name = `windmill_publication_${random_adj()}`
															relations = [{ schema_name: 'public', table_to_track: [] }]
															return
														}

														publication_name = ''
														relations = []
														transaction_to_track = []
													}}
													let:item
												>
													<ToggleButton value="create" label="Create Publication" {item} />
													<ToggleButton value="get" label="Get Publication" {item} />
												</ToggleButtonGroup>
												{#if selectedPublicationAction === 'create'}
													<div class="flex gap-3">
														<input
															disabled={!can_write}
															type="text"
															bind:value={publication_name}
															placeholder={'Publication Name'}
														/>
														<Button
															color="light"
															size="xs"
															variant="border"
															disabled={emptyStringTrimmed(publication_name) ||
																(relations && relations.length === 0) ||
																!can_write}
															on:click={createPublication}>Create</Button
														>
													</div>
												{:else}
													<PublicationPicker
														{can_write}
														{postgres_resource_path}
														bind:transaction_to_track
														bind:relations
														bind:items={publicationItems}
														bind:publication_name
													/>
												{/if}
												<RelationPicker {can_write} bind:relations {postgresVersion} />
											</div>
										</Section></div
									>
								</TabContent>
							</div>
						</svelte:fragment>
					</Tabs>
				</Label>
			{/if}
		</div>
	</Section>
</div>
