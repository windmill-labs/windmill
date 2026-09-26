<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Toggle from '../Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'instance_pg_disabled'

	let saving = $state(false)
	// Absent means on, which is what every instance that predates the setting expects.
	let enabled = $derived(!$values[KEY])

	async function setEnabled(next: boolean) {
		saving = true
		try {
			await SettingService.setGlobal({ key: KEY, requestBody: { value: next ? null : true } })
			$values[KEY] = next ? undefined : true
			sendUserToast(
				next
					? "Windmill's database can back data tables and Ducklake catalogs again"
					: "Windmill's database is no longer offered for new data tables and Ducklake catalogs"
			)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			saving = false
		}
	}
</script>

<div class="flex flex-col gap-4">
	{#if isCloudHosted()}
		<Alert type="info" title="Not available on cloud" size="xs">
			On Windmill Cloud, data tables and Ducklake catalogs use a PostgreSQL resource or the external
			instance cluster.
		</Alert>
	{:else}
		<Toggle
			{disabled}
			checked={enabled}
			options={{
				right: 'Data tables and Ducklake catalogs can use Windmill’s own database'
			}}
			id="instance-pg-enabled"
			on:change={({ detail }) => !saving && setEnabled(detail)}
		/>
		<p class="text-xs text-secondary max-w-prose">
			Turning this off leaves what already runs on it alone: existing data tables and catalogs keep
			resolving, and their databases stay where they are. What it stops is a workspace naming
			Windmill's database for something new, so the external cluster becomes the only substrate
			Windmill administers.
		</p>
	{/if}
</div>
