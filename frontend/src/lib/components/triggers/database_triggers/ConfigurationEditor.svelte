<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Required from '$lib/components/Required.svelte'
	import { DatabaseTriggerService, type Relations, type TransactionType } from '$lib/gen'
	import { emptyString, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Plus, Save, X } from 'lucide-svelte'
	import MultiSelect from 'svelte-multiselect'
	import { fade } from 'svelte/transition'
	import { tick } from 'svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PublicationPicker from './PublicationPicker.svelte'
	import { workspaceStore } from '$lib/stores'
	import SlotPicker from './SlotPicker.svelte'

	export let database_resource_path: string = ''
	export let publication_name: string = ''
	export let replication_slot_name: string = ''
	export let relations: Relations[] = []
	type actions = 'create' | 'get'
	let drawer: Drawer
	let drawerLoading = true
	let open = false
	let selected: actions = 'create'
	let selectedSlotAction: actions = 'create'
	let items: string[] = []
	let transactionType: TransactionType[] = ['Insert', 'Update', 'Delete']
	let selectedTable: 'all' | 'specific' = 'specific'
	export let transaction_to_track: TransactionType[] = []

	export async function openNew(edit: boolean) {
		open = true
		selected = 'create'
		items = []
		selectedTable = 'specific'
		await tick()
		if (edit) {
			selected = 'get'
		}
		drawerLoading = true
		try {
			drawer?.openDrawer()
		} finally {
			drawerLoading = false
		}
	}

	async function createPublication() {
		try {
			const message = await DatabaseTriggerService.createDatabasePublication({
				path: database_resource_path,
				publication: publication_name,
				workspace: $workspaceStore!,
				requestBody: {
					transaction_to_track,
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
			const message = await DatabaseTriggerService.createDatabaseSlot({
				path: database_resource_path,
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

	async function configurationSet(): Promise<void> {
		drawer.closeDrawer()
	}
</script>

{#if open}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent title={'Database Configuration'} on:close={drawer.closeDrawer}>
			<svelte:fragment slot="actions">
				{#if !drawerLoading}
					<Button
						startIcon={{ icon: Save }}
						disabled={emptyString(replication_slot_name) || emptyString(publication_name)}
						on:click={configurationSet}
					>
						Save
					</Button>
				{/if}
			</svelte:fragment>
			{#if drawerLoading}
				<Loader2 class="animate-spin" />
			{:else}
				<div class="flex flex-col gap-12 mt-6">
					<Section label="Transactions">
						<p class="text-xs mb-1 text-tertiary">
							Choose what kind of database transaction you want to track allowed operations are
							<strong>Insert</strong>, <strong>Update</strong>, <strong>Delete</strong>
							<Required required={true} />
						</p>

						<MultiSelect
							options={transactionType}
							bind:selected={transaction_to_track}
							duplicates={false}
							liOptionClass={'box'}
						/>
					</Section>

					<Section label="Slot name">
						<div class="flex flex-col gap-3">
							<ToggleButtonGroup bind:selected={selectedSlotAction}>
								<ToggleButton value="create" label="Create Slot" />
								<ToggleButton value="get" label="Get Slot" />
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
										disabled={emptyString(replication_slot_name)}
										on:click={createSlot}>Create</Button
									>
								</div>
							{:else}
								<SlotPicker {database_resource_path} bind:replication_slot_name />
							{/if}
						</div>
					</Section>

					<Section label="Publication">
						<div class="flex flex-col gap-3">
							<ToggleButtonGroup
								bind:selected
								on:selected={() => {
									publication_name = ''
									relations = []
									transaction_to_track = []
								}}
							>
								<ToggleButton value="create" label="Create Publication" />
								<ToggleButton value="get" label="Get Publication" />
							</ToggleButtonGroup>

							{#if selected === 'create'}
								<div class="flex gap-3">
									<input
										type="text"
										bind:value={publication_name}
										placeholder={'Publication Name'}
									/>
									<Button
										color="light"
										size="xs"
										variant="border"
										disabled={emptyString(publication_name)}
										on:click={createPublication}>Create</Button
									>
								</div>
							{:else}
								<PublicationPicker
									{database_resource_path}
									bind:transaction_to_track
									bind:table_to_track={relations}
									bind:items
									bind:publication_name
									bind:selectedTable
								/>
							{/if}

							<ToggleButtonGroup bind:selected={selectedTable}>
								<ToggleButton value="all" label="All Tables" />
								<ToggleButton value="specific" label="Specific Tables" />
							</ToggleButtonGroup>

							{#if selectedTable != 'all' && (selected === 'create' || (items.length > 0 && !emptyString(publication_name)))}
								<div class="flex flex-col gap-4">
									{#if relations && relations.length > 0}
										{#each relations as v, i}
											<div class="flex w-full gap-2 items-center">
												<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
													<label class="flex flex-col w-full">
														<div class="text-secondary text-sm mb-2">Schema Name</div>
														<input type="text" bind:value={v.schema_name} />
													</label>
													{#each v.table_to_track as table_to_track, j}
														<div class="flex w-full gap-2 items-center p-5">
															<div class="rounded shadow-inner p-2 flex w-full flex-col gap-4 mt-1">
																<label class="flex flex-col w-full">
																	<div class="text-secondary text-sm mb-2">Table Name</div>
																	<input type="text" bind:value={table_to_track.table_name} />
																</label>
																<!-- svelte-ignore a11y-label-has-associated-control -->
																<label class="flex flex-col w-full">
																	<div class="text-secondary text-sm mb-2">Columns</div>
																	<MultiSelect
																		options={table_to_track.columns_name ?? []}
																		allowUserOptions="append"
																		bind:selected={table_to_track.columns_name}
																		noMatchingOptionsMsg=""
																		createOptionMsg={null}
																		duplicates={false}
																	/>
																</label>
																<label class="flex flex-col w-full">
																	<div class="text-secondary text-sm mb-2">Where Clause</div>
																	<input type="text" bind:value={table_to_track.where_clause} />
																</label>
																<button
																	transition:fade|local={{ duration: 100 }}
																	class="rounded items-center p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
																	aria-label="Clear"
																	on:click={() => {
																		v.table_to_track = v.table_to_track.filter(
																			(_, index) => index !== j
																		)
																	}}
																>
																	Remove
																</button>
															</div>
														</div>
													{/each}
													<Button
														variant="border"
														color="light"
														size="xs"
														btnClasses="mt-1"
														on:click={() => {
															if (
																relations[i].table_to_track == undefined ||
																!Array.isArray(relations[i].table_to_track)
															) {
																relations[i].table_to_track = []
															}
															relations[i].table_to_track = relations[i].table_to_track.concat({
																table_name: '',
																columns_name: []
															})
														}}
														startIcon={{ icon: Plus }}
													>
														Add Table
													</Button>
												</div>
												<button
													transition:fade|local={{ duration: 100 }}
													class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
													aria-label="Clear"
													on:click={() => {
														relations = relations.filter((_, index) => index !== i)
													}}
												>
													<X size={14} />
												</button>
											</div>
										{/each}
									{/if}
									<div class="flex items-baseline">
										<Button
											variant="border"
											color="light"
											size="xs"
											btnClasses="mt-1"
											on:click={() => {
												if (relations == undefined || !Array.isArray(relations)) {
													relations = []
												}
												relations = relations.concat({
													schema_name: '',
													table_to_track: []
												})
											}}
											startIcon={{ icon: Plus }}
										>
											Add Schema
										</Button>
									</div>
								</div>
							{/if}
						</div>
					</Section>
				</div>
			{/if}
		</DrawerContent>
	</Drawer>
{/if}
