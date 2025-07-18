<script module lang="ts">
	export type DucklakeSettingsType = {
		ducklakes: {
			name: string
			catalog: {
				resource_type: 'postgresql' | 'mysql'
				resource_path?: string
			}
			storage: {
				storage?: string
				path: string
			}
		}[]
	}
</script>

<script>
	import { Plus } from 'lucide-svelte'

	import { ClearableInput } from '../common'
	import Button from '../common/button/Button.svelte'

	import Description from '../Description.svelte'
	import { random_adj } from '../random_positive_adjetive'
	import { DataTable, Cell, Row } from '../table'
	import Head from '../table/Head.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Select from '../select/Select.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { SettingService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { safeSelectItems } from '../select/utils.svelte'

	type Props = {
		ducklakeSettings: DucklakeSettingsType
	}
	let { ducklakeSettings = $bindable() }: Props = $props()

	function onNewDucklake() {
		const name = ducklakeSettings.ducklakes.length ? `${random_adj()}_ducklake` : 'main'
		ducklakeSettings.ducklakes.push({
			name,
			catalog: {
				resource_type: 'postgresql',
				resource_path: undefined
			},
			storage: {
				storage: undefined,
				path: ''
			}
		})
	}
	function removeDucklake(index: number) {
		ducklakeSettings.ducklakes.splice(index, 1)
	}

	let secondaryStorageNames = usePromise(
		() => SettingService.getSecondaryStorageNames({ workspace: $workspaceStore! }),
		{ loadInit: false }
	)
	$effect(() => {
		$workspaceStore
		secondaryStorageNames.refresh()
	})

	let tableHeadNames = ['Name', 'Catalog', 'Workspace storage', '']
</script>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold">Ducklake</div>
		<Description link="https://www.windmill.dev/docs/core_concepts/ducklake">
			Windmill has first class support for Ducklake. You can reference a ducklake in your DuckDB
			scripts with the
			<code>ATTACH 'ducklake:name'</code> syntax
		</Description>
	</div>
</div>

<DataTable>
	<Head>
		<tr>
			{#each tableHeadNames as name, i}
				<Cell head first={i == 0} last={i == tableHeadNames.length - 1}>{name}</Cell>
			{/each}
		</tr>
	</Head>
	<tbody class="divide-y bg-surface">
		{#if ducklakeSettings.ducklakes.length == 0}
			<Row>
				<Cell colspan={tableHeadNames.length} class="text-center">
					No ducklake in this workspace yet
				</Cell>
			</Row>
		{/if}
		{#each ducklakeSettings.ducklakes as ducklake, ducklakeIndex}
			<Row>
				<Cell first class="w-56">
					<ClearableInput bind:value={ducklake.name} placeholder="Name" />
				</Cell>
				<Cell>
					<div class="flex gap-4">
						<Select
							items={[
								{ value: 'postgresql', label: 'PostgreSQL' },
								{ value: 'mysql', label: 'MySQL' }
							]}
							bind:value={ducklake.catalog.resource_type}
							class="w-36"
						/>
						<ResourcePicker
							bind:value={ducklake.catalog.resource_path}
							resourceType={ducklake.catalog.resource_type}
						/>
					</div>
				</Cell>
				<Cell>
					<div class="flex gap-4">
						<Select
							items={safeSelectItems(secondaryStorageNames.value)}
							bind:value={ducklake.storage.storage}
							class="w-36"
						/>
						<input placeholder="Data path" />
					</div>
				</Cell>
				<Cell class="w-12">
					<CloseButton small on:close={() => removeDucklake(ducklakeIndex)} />
				</Cell>
			</Row>
		{/each}
		<Row class="!border-0">
			<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
				<div class="flex justify-center">
					<Button size="sm" btnClasses="max-w-fit" variant="border" on:click={onNewDucklake}>
						<Plus /> New ducklake
					</Button>
				</div>
			</Cell>
		</Row>
	</tbody>
</DataTable>
