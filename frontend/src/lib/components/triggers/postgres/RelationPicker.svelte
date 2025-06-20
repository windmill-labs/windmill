<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { Relations } from '$lib/gen'
	import { Plus, Trash } from 'lucide-svelte'
	import { getDefaultTableToTrack, invalidRelations } from './utils'
	import AddPropertyFormV2 from '$lib/components/schema/AddPropertyFormV2.svelte'
	import Label from '$lib/components/Label.svelte'
	import { emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	interface Props {
		relations?: Relations[] | undefined
		can_write?: boolean
		disabled?: boolean
		pg14?: boolean
	}

	let {
		relations = $bindable(undefined),
		can_write = true,
		disabled = false,
		pg14 = $bindable(false)
	}: Props = $props()

	let selected = $derived(relations && relations.length > 0 ? 'specific' : 'all')

	let cached: Relations[] | undefined = $state(relations)

	$effect(() => {
		if (pg14 && relations) {
			relations.forEach((relation) => {
				if (relation.table_to_track.length === 0) {
					relation.table_to_track.push({ table_name: '' })
				} else {
					relation.table_to_track.forEach((table_to_track) => {
						if (table_to_track.columns_name) {
							table_to_track.columns_name = undefined
						}
						if (!emptyStringTrimmed(table_to_track.where_clause)) {
							table_to_track.where_clause = undefined
						}
					})
				}
			})
		}
	})

	function addTable(name: string, index: number) {
		if (relations) {
			relations[index].table_to_track = relations[index].table_to_track.concat({
				table_name: name,
				columns_name: []
			})
		}
	}

	function updateRelationsFor(i: number, updateFn: (r: Relations) => Relations) {
		if (relations) {
			relations = relations.map((r, index) => {
				if (i === index) {
					r = updateFn(r)
				}
				return r
			})
		}
	}
</script>

<div class="flex flex-col gap-4 h-full">
	<div class="grow-0 flex flex-row gap-2 w-full items-center">
		<div class="grow-0">
			<ToggleButtonGroup
				on:selected={({ detail }) => {
					if (detail === 'all') {
						cached = relations
						relations = undefined
					} else {
						if (!cached || cached.length === 0) {
							if (!relations || relations.length == 0) {
								relations = getDefaultTableToTrack(pg14)
								cached = relations
							} else {
								cached = relations
							}
						} else {
							relations = cached
						}
					}
				}}
				bind:selected
				{disabled}
			>
				{#snippet children({ item })}
					<ToggleButton value="all" label="All Tables" {item} />
					<ToggleButton value="specific" label="Specific Tables" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>
	</div>

	{#if relations && relations.length > 0}
		<div class="flex flex-col gap-4 grow min-h-0 overflow-y-auto">
			{#each relations as v, i}
				<div class="flex w-full gap-3 items-center">
					<div class="w-full flex flex-col gap-2 border py-2 px-4 rounded-md">
						<Label label="Schema Name" required class="w-full">
							{#snippet header()}
								<Tooltip small>
									<p>
										Enter the name of the <strong>schema</strong> that contains the table(s) you
										want to track.
										<br />If no tables are added, all tables within the <strong>schema</strong> will
										be tracked for the selected transactions (insert, update, delete).
									</p>
								</Tooltip>
							{/snippet}

							<input class="mt-1" type="text" bind:value={v.schema_name} {disabled} />
						</Label>
						<div class="flex flex-col w-full gap-4 items-center p-5">
							{#each v.table_to_track as table_to_track, j}
								<div
									class="relative rounded bg-surface-disabled p-2 flex w-full flex-col gap-4 group"
								>
									<Label label="Table Name" required>
										{#snippet header()}
											<Tooltip small>Enter the name of the table you want to track.</Tooltip>
										{/snippet}
										<input
											type="text"
											bind:value={table_to_track.table_name}
											class="!bg-surface mt-1"
											{disabled}
										/>
									</Label>
									<Label label="Columns">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#selecting-specific-columns"
												small
											>
												<p>
													Enter the names of the <strong>columns</strong> you want to track.
													<br />If no columns are specified, all columns in the table will be
													tracked for the selected transactions (insert, update, delete).
												</p>
												<p class="text-xs text-gray-500 mt-1">
													<strong class="font-semibold">Note:</strong>
													<br />
													-
													<strong
														>Column-specific tracking is only supported in PostgreSQL 15 and above.</strong
													>
													<br />
													- In PostgreSQL 14, all columns will be tracked automatically and selective
													tracking will be disabled.
													<br />
													- If your trigger contains <strong>UPDATE</strong> or
													<strong>DELETE</strong>
													transactions, the row filter WHERE clause must contain only columns covered
													by the <strong>replica identity</strong> (see REPLICA IDENTITY).
													<br />
													- If your trigger contains only <strong>INSERT</strong> transactions, the row
													filter WHERE clause can use any column.
												</p>
											</Tooltip>
										{/snippet}
										<div class="mt-1">
											<MultiSelect
												items={safeSelectItems(table_to_track.columns_name ?? [])}
												placeholder="Select columns"
												disabled={pg14}
												bind:value={
													() => table_to_track.columns_name ?? [],
													(columns_name) => {
														updateRelationsFor(i, (rel) => ({
															...rel,
															table_to_track: rel.table_to_track.map((t, idx) =>
																idx !== j ? t : { ...t, columns_name }
															)
														}))
													}
												}
												onCreateItem={(x) =>
													updateRelationsFor(i, (rel) => ({
														...rel,
														table_to_track: rel.table_to_track.map((t, idx) =>
															idx !== j ? t : { ...t, columns_name: [...(t.columns_name ?? []), x] }
														)
													}))}
											/>
										</div>
									</Label>
									<Label label="Where Clause">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#filtering-rows-with-where-condition"
												small
											>
												<p class="text-sm">
													Use this field to define a row filter for the selected table. The <strong
														>WHERE</strong
													> clause allows only simple expressions and cannot include user-defined functions,
													operators, types, collations, system column references, or non-immutable built-in
													functions.
												</p>
												<p class="text-xs text-gray-500 mt-1">
													<strong class="font-semibold">Note:</strong>
													<br />
													-
													<strong
														>Row filtering with WHERE clauses is only supported in PostgreSQL 15 and
														above.</strong
													>
													<br />
													- In PostgreSQL 14, row filtering is not available and this field is disabled.
													<br />
													- If your trigger contains <strong>UPDATE</strong> or
													<strong>DELETE</strong>
													transactions, the row filter WHERE clause must contain only columns covered
													by the <strong>replica identity</strong> (see REPLICA IDENTITY).
													<br />
													- If your trigger contains only <strong>INSERT</strong> transactions, the row
													filter WHERE clause can use any column.
												</p>
											</Tooltip>
										{/snippet}
										<input
											disabled={pg14}
											type="text"
											bind:value={table_to_track.where_clause}
											class="!bg-surface mt-1"
										/>
									</Label>
									<Button
										variant="border"
										wrapperClasses="absolute -top-3 -right-3"
										btnClasses="hidden group-hover:block hover:bg-red-500 hover:text-white p-2 rounded-full hover:border-red-500 transition-all duration-300"
										color="light"
										size="xs"
										on:click={() => {
											if (pg14 && v.table_to_track.length > 1) {
												updateRelationsFor(i, (r) => ({
													...r,
													table_to_track: r.table_to_track.filter((_, idx) => idx !== j)
												}))
											} else if (!pg14) {
												updateRelationsFor(i, (r) => ({
													...r,
													table_to_track: r.table_to_track.filter((_, idx) => idx !== j)
												}))
											}
										}}
										iconOnly
										startIcon={{ icon: Trash }}
										{disabled}
									/>
								</div>
							{/each}
							<AddPropertyFormV2
								customName="Table"
								on:add={({ detail }) => {
									addTable(detail.name, i)
								}}
								{disabled}
							>
								{#snippet trigger()}
									<Button
										wrapperClasses="w-full border border-dashed rounded-md"
										color="light"
										size="xs"
										{disabled}
										startIcon={{ icon: Plus }}
										nonCaptureEvent
									>
										Add table
									</Button>
								{/snippet}
							</AddPropertyFormV2>
						</div>
					</div>
					<Button
						variant="border"
						color="light"
						size="xs"
						btnClasses="bg-surface-secondary hover:bg-red-500 hover:text-white p-2 rounded-full"
						aria-label="Clear"
						on:click={() => {
							if (relations) {
								relations = relations.filter((_, index) => index !== i)
							}
						}}
						{disabled}
					>
						<Trash size={14} />
					</Button>
				</div>
			{/each}
		</div>
	{/if}
	{#if selected === 'specific'}
		<div class="grow min-w-0 pr-10">
			<AddPropertyFormV2
				customName="Schema"
				on:add={({ detail }) => {
					if (relations == undefined || !Array.isArray(relations)) {
						relations = getDefaultTableToTrack(pg14)
					} else if (emptyStringTrimmed(detail.name)) {
						sendUserToast('Schema name must not be empty', true)
					} else {
						if (
							invalidRelations(relations, {
								showError: true,
								trackSchemaTableError: false
							}) === ''
						) {
							relations = relations.concat({
								schema_name: detail.name,
								table_to_track: pg14 ? [{ table_name: '' }] : []
							})
						}
					}
				}}
				{disabled}
			>
				{#snippet trigger()}
					<Button
						variant="border"
						color="light"
						size="xs"
						btnClasses="w-full"
						disabled={!can_write || disabled}
						startIcon={{ icon: Plus }}
						nonCaptureEvent
					>
						Add schema
					</Button>
				{/snippet}
			</AddPropertyFormV2>
		</div>
	{/if}
</div>
