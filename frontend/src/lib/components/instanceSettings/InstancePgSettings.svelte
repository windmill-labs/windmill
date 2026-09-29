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
		markSettingSaved?: (key: string) => void
	}

	let { values, disabled = false, markSettingSaved }: Props = $props()

	const KEY = 'instance_pg_disabled'

	let saving = $state(false)
	// Absent means on, which is what every instance that predates the setting expects.
	let enabled = $derived(!$values[KEY])

	async function setEnabled(next: boolean) {
		saving = true
		try {
			await SettingService.setGlobal({ key: KEY, requestBody: { value: next ? null : true } })
			$values[KEY] = next ? undefined : true
			markSettingSaved?.(KEY)
			sendUserToast(
				next
					? "Windmill's database can back data tables and Ducklake catalogs again"
					: "Windmill's database is off: its data tables no longer resolve, and it is not offered for new ones"
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
			Turning this off takes Windmill's database out of use: data tables on it stop resolving, so
			jobs, apps and triggers using them fail until they are moved or this is turned back on, and no
			workspace can name it for a new data table or Ducklake catalog. The databases themselves stay
			where they are. Ducklake catalogs already on it keep working.
		</p>
	{/if}
</div>
