<script lang="ts">
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { onMount } from 'svelte'
	import type { Setting } from './settings'
	import { Button } from '$lib/components/common'
	import { SettingService } from '$lib/gen'

	onMount(() => {})

	export const settings: Record<string, Setting[]> = {
		Core: [
			{
				label: 'Base Url',
				key: 'base_url',
				fieldType: 'text',
				placeholder: 'https://windmill.com'
			},
			{
				label: 'Request Size Limit',
				key: 'request_size_limit_mb',
				fieldType: 'number',
				placeholder: '50'
			},
			{
				label: 'Retention Period',
				key: 'retention_period_days',
				fieldType: 'number',
				placeholder: '6 0'
			}
		],
		SMTP: [
			{
				label: 'Host',
				key: 'smtp_host',
				fieldType: 'text',
				placeholder: 'smtp.gmail.com'
			},
			{
				label: 'Port',
				key: 'smtp_port',
				fieldType: 'number',
				placeholder: '567'
			},
			{
				label: 'Username',
				key: 'smtp_username',
				fieldType: 'text',
				placeholder: 'smtp.gmail.com'
			},
			{
				label: 'Password',
				key: 'smtp_password',
				fieldType: 'text'
			},
			{
				label: 'Implicit TLS',
				key: 'smtp_implicit_tls',
				fieldType: 'boolean'
			}
		]
	}

	let values: Setting[] | undefined = undefined

	loadSettings()
	async function loadSettings() {
		values = Object.fromEntries(
			(
				await Promise.all(
					Object.entries(settings).map(
						async ([_, y]) =>
							await Promise.all(
								y.map(async (x) => [x.key, await SettingService.getGlobal({ key: x.key })])
							)
					)
				)
			).flat()
		)
	}
</script>

<CenteredModal title="Instance Settings">
	<div class="flex pb-6">
		<Button variant="border" size="sm" href="/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
	</div>
	<div class="flex-col flex gap-4">
		{#each Object.keys(settings) as category}
			<div class="text-primary font-semibold text-lg">{category}</div>
			<div class="flex-col flex gap-4">
				{#each settings[category] as setting}
					<label class="block pb-2">
						<span class="text-primary font-semibold"> {setting.label} </span>
						{#if setting.fieldType == 'text' || setting.fieldType == 'number'}
							{#if values}
								{#if setting.fieldType == 'text'}
									<input
										type="text"
										placeholder={setting.placeholder}
										bind:value={values[setting.key]}
									/>
								{:else if setting.fieldType == 'number'}
									<input
										type="number"
										placeholder={setting.placeholder}
										bind:value={values[setting.key]}
									/>
								{:else if setting.fieldType == 'password'}
									<input
										type="password"
										placeholder={setting.placeholder}
										bind:value={values[setting.key]}
									/>
								{/if}
							{:else}
								<input disabled placeholder="Loading..." />
							{/if}
						{/if}
					</label>
				{/each}
			</div>
		{/each}
	</div>
</CenteredModal>
