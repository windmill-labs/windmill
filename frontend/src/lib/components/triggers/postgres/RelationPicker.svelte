<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { Relations } from '$lib/gen'
	import { Plus, X } from 'lucide-svelte'
	import MultiSelect from 'svelte-multiselect'
	import { fade } from 'svelte/transition'
	export let relations: Relations[] = []
	export let selectedTable: 'all' | 'specific'
</script>

<div class="flex flex-col gap-4">
	<ToggleButtonGroup bind:selected={selectedTable}>
		<ToggleButton value="all" label="All Tables" />
		<ToggleButton value="specific" label="Specific Tables" />
	</ToggleButtonGroup>
	{#if selectedTable !== 'all'}
		{#if relations && relations.length > 0}
			{#each relations as v, i}
				<div class="flex w-full gap-2 items-center">
					<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
						<label class="flex flex-col w-full">
							<div class="text-secondary text-sm mb-2 flex flex-row gap-1"
								><p>Schema Name</p><Tooltip
									><p>
										Enter the name of the <strong>schema</strong> that contains the table(s) you
										want to track.
										<br />If no tables are added, all tables within the <strong>schema</strong> will
										be tracked for the selected transactions (insert, update, delete).
									</p></Tooltip
								></div
							>
							<input type="text" bind:value={v.schema_name} />
						</label>
						{#each v.table_to_track as table_to_track, j}
							<div class="flex w-full gap-2 items-center p-5">
								<div class="rounded shadow-inner p-2 flex w-full flex-col gap-4 mt-1">
									<label class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2 flex flex-row gap-1"
											><p>Table Name</p><Tooltip
												>Enter the name of the table you want to track.</Tooltip
											></div
										>
										<input type="text" bind:value={table_to_track.table_name} />
									</label>
									<!-- svelte-ignore a11y-label-has-associated-control -->
									<label class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2 flex flex-row gap-1"
											><p>Columns</p><Tooltip
												><p>
													Enter the names of the <strong>columns</strong> you want to track.
													<br />If no columns are specified, all columns in the table will be
													tracked for the selected transactions (insert, update, delete).
												</p>
												<p class="text-xs text-gray-500 mt-1">
													<strong class="font-semibold">Note:</strong>
													<br />- If your trigger contains <strong>UPDATE</strong> or
													<strong>DELETE</strong>
													transactions, the row filter WHERE clause must contain only columns covered
													by the <strong>replica identity</strong> (see REPLICA IDENTITY).
													<br />- If your trigger contains only <strong>INSERT</strong> transactions,
													the row filter WHERE clause can use any column.
												</p></Tooltip
											></div
										>
										<p class="text-xs mb-3 text-tertiary">
											Enter the names of the <strong>columns</strong> you want to track.
											<br />If no columns are specified, all columns in the table will be tracked.
										</p>
										<MultiSelect
											options={table_to_track.columns_name ?? []}
											allowUserOptions="append"
											bind:selected={table_to_track.columns_name}
											ulOptionsClass={'!bg-surface-secondary'}
											noMatchingOptionsMsg=""
											createOptionMsg={null}
											duplicates={false}
										/>
									</label>
									<label class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2 flex flex-row gap-1"
											><p>Where Clause</p><Tooltip>
												<p class="text-sm">
													Use this field to define a row filter for the selected table. The <strong
														>WHERE</strong
													> clause allows only simple expressions and cannot include user-defined functions,
													operators, types, collations, system column references, or non-immutable built-in
													functions.
												</p>
												<p class="text-xs text-gray-500 mt-1">
													<strong class="font-semibold">Note:</strong>
													<br />- If your trigger contains <strong>UPDATE</strong> or
													<strong>DELETE</strong>
													transactions, the row filter WHERE clause must contain only columns covered
													by the <strong>replica identity</strong> (see REPLICA IDENTITY).
													<br />- If your trigger contains only <strong>INSERT</strong> transactions,
													the row filter WHERE clause can use any column.
												</p>
											</Tooltip></div
										>
										<input type="text" bind:value={table_to_track.where_clause} />
									</label>
									<button
										transition:fade|local={{ duration: 100 }}
										class="rounded items-center p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
										aria-label="Clear"
										on:click={() => {
											v.table_to_track = v.table_to_track.filter((_, index) => index !== j)
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
	{/if}
</div>
