<script module lang="ts">
	import type { GetSettingsResponse } from '$lib/gen'

	export type DbtWarehouse = { name: string; resource_path?: string; target?: string }
	export type DbtSettingsType = { warehouses: DbtWarehouse[] }

	/** The name a project reaches when its descriptor names none. */
	export const DEFAULT_WAREHOUSE = 'main'

	/**
	 * The resource types a warehouse may point at: `dbt_profile`, which carries a
	 * `profiles.yml` block verbatim and so reaches any adapter, plus the connection types
	 * `render_profile` (backend/windmill-worker/src/dbt_profiles.rs) translates. Anything
	 * else has no way to become a dbt target at all.
	 */
	export const WAREHOUSE_RESOURCE_TYPES = [
		'dbt_profile',
		'postgresql',
		'redshift',
		'mysql',
		'snowflake',
		'snowflake_oauth',
		'bigquery',
		'gcp_service_account',
		'databricks'
	].join(',')

	export function convertDbtSettingsFromBackend(
		settings: GetSettingsResponse['dbt_warehouses']
	): DbtSettingsType {
		return {
			warehouses: Object.entries(settings ?? {}).map(([name, rest]) => ({
				name,
				resource_path: (rest as DbtWarehouse)?.resource_path,
				target: (rest as DbtWarehouse)?.target
			}))
		}
	}

	export function convertDbtSettingsToBackend(
		settings: DbtSettingsType
	): Record<string, { resource_path: string; target?: string }> {
		// Null-prototype: the backend accepts any alphanumeric name, `constructor`
		// and `toString` among them, and on a plain object those are inherited —
		// `in` would call them duplicates, and assigning `__proto__` would set the
		// prototype instead of a key.
		const out: Record<string, { resource_path: string; target?: string }> = Object.create(null)
		for (const w of settings.warehouses) {
			const name = w.name?.trim()
			if (!name) throw 'A warehouse needs a name'
			if (name in out) throw 'Settings contain duplicate warehouse name: ' + name
			if (!w.resource_path) throw 'No resource selected for ' + name
			out[name] = { resource_path: w.resource_path, target: w.target || undefined }
		}
		return out
	}
</script>

<script lang="ts">
	import { Plus, Trash } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import TextInput from '../text_input/TextInput.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import Cell from '../table/Cell.svelte'
	import SettingsFooter from './SettingsFooter.svelte'
	import Description from '$lib/components/Description.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clone } from '$lib/utils'

	let {
		dbtSettings = $bindable(),
		dbtSavedSettings = $bindable(),
		onSave: onSaveProp = undefined,
		onDiscard = undefined
	}: {
		dbtSettings: DbtSettingsType
		dbtSavedSettings: DbtSettingsType
		onSave?: () => void
		onDiscard?: () => void
	} = $props()

	let hasUnsavedChanges = $derived(
		JSON.stringify(dbtSettings.warehouses) !== JSON.stringify(dbtSavedSettings.warehouses)
	)

	async function onSave() {
		try {
			const dbt_warehouses = convertDbtSettingsToBackend(dbtSettings)
			await WorkspaceService.editDbtWarehouses({
				workspace: $workspaceStore!,
				requestBody: { dbt_warehouses }
			})
			dbtSavedSettings = clone(dbtSettings)
			sendUserToast('dbt warehouses saved successfully')
			onSaveProp?.()
		} catch (e) {
			sendUserToast(e, true)
			throw e
		}
	}
</script>

<Description link="https://www.windmill.dev/docs/getting_started/scripts_quickstart/dbt">
	Where dbt projects in this workspace run. A project names a warehouse by name in its descriptor (<span
		class="font-mono">profile.warehouse</span
	>) and reaches
	<span class="font-mono">{DEFAULT_WAREHOUSE}</span> when it names none, so a project carries no
	connection of its own. The name is also what its tables are keyed on in the asset graph (<span
		class="font-mono">dbt://{DEFAULT_WAREHOUSE}/schema/table</span
	>), so two projects on one warehouse share their nodes. Each entry points either at one of
	Windmill's own connection resources, whose fields are translated into a dbt target, or at a
	<span class="font-mono">dbt_profile</span> resource, which carries a
	<span class="font-mono">profiles.yml</span> target as it is and so reaches any adapter dbt has. Configuring
	one here is what makes it available: anyone who may run a dbt script builds with it and reads its models,
	without being granted the resource, the same bargain workspace object storage makes.
</Description>

<DataTable>
	<Head>
		<tr>
			<Cell head first>Name</Cell>
			<Cell head>Resource</Cell>
			<Cell head>Target</Cell>
			<Cell head last />
		</tr>
	</Head>
	<tbody class="divide-y">
		{#each dbtSettings.warehouses as warehouse, i (i)}
			<Row>
				<Cell first>
					<TextInput
						bind:value={warehouse.name}
						inputProps={{ placeholder: DEFAULT_WAREHOUSE }}
						class="min-w-32"
					/>
				</Cell>
				<Cell>
					<ResourcePicker
						class="min-w-48"
						bind:value={warehouse.resource_path}
						resourceType={WAREHOUSE_RESOURCE_TYPES}
						placeholder="warehouse resource"
					/>
				</Cell>
				<Cell>
					<TextInput
						bind:value={warehouse.target}
						inputProps={{ placeholder: 'default' }}
						class="min-w-24"
					/>
				</Cell>
				<Cell last>
					<Button
						size="xs"
						color="light"
						variant="border"
						startIcon={{ icon: Trash }}
						iconOnly
						on:click={() => {
							dbtSettings.warehouses = dbtSettings.warehouses.filter((_, j) => j !== i)
						}}
					/>
				</Cell>
			</Row>
		{/each}
		<Row>
			<Cell first colspan={4}>
				<Button
					size="sm"
					btnClasses="max-w-fit"
					variant="default"
					on:click={() => {
						dbtSettings.warehouses = [
							...dbtSettings.warehouses,
							{ name: dbtSettings.warehouses.length === 0 ? DEFAULT_WAREHOUSE : '' }
						]
					}}
				>
					<Plus /> New warehouse
				</Button>
			</Cell>
		</Row>
	</tbody>
</DataTable>

<SettingsFooter
	class="mt-6 mb-16"
	inline
	{hasUnsavedChanges}
	{onSave}
	onDiscard={() => onDiscard?.()}
	saveLabel="Save dbt warehouses"
/>
