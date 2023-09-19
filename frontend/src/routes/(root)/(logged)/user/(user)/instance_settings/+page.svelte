<script lang="ts">
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import type { Setting, SettingStorage } from './settings'
	import { Button } from '$lib/components/common'
	import { ConfigService, SettingService } from '$lib/gen'
	import Toggle from '$lib/components/Toggle.svelte'
	import SecondsInput from '$lib/components/common/seconds/SecondsInput.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import OAuthSetting from '$lib/components/OAuthSetting.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	export const settings: Record<string, Setting[]> = {
		Core: [
			{
				label: 'Base Url',
				key: 'base_url',
				fieldType: 'text',
				placeholder: 'https://windmill.com',
				storage: 'setting'
			},
			{
				label: 'Request Size Limit In MB',
				key: 'request_size_limit_mb',
				fieldType: 'number',
				placeholder: '50',
				storage: 'setting'
			},
			{
				label: 'Retention Period in secs',
				key: 'retention_period_secs',
				fieldType: 'seconds',
				placeholder: '60',
				storage: 'setting'
			}
		],
		SMTP: [
			{
				label: 'Host',
				key: 'smtp_host',
				fieldType: 'text',
				placeholder: 'smtp.gmail.com',
				storage: 'smtp'
			},
			{
				label: 'Port',
				key: 'smtp_port',
				fieldType: 'number',
				placeholder: '567',
				storage: 'smtp'
			},
			{
				label: 'Username',
				key: 'smtp_username',
				fieldType: 'text',
				placeholder: 'ruben@windmill.dev',
				storage: 'smtp'
			},
			{
				label: 'Password',
				key: 'smtp_password',
				fieldType: 'password',
				storage: 'smtp'
			},
			{
				label: 'Implicit TLS',
				key: 'smtp_implicit_tls',
				fieldType: 'boolean',
				storage: 'smtp'
			}
		]
	}

	let values: Setting[] | undefined = undefined
	let serverConfig: {
		smtp?: Record<string, any>
	} = {}

	loadSettings()
	async function loadSettings() {
		try {
			serverConfig = (await ConfigService.getConfig({ name: 'server' })) ?? {}
		} catch (e) {
			console.log("Sever config not found, assuming it's first setup")
		}

		async function getValue(key: string, storage: SettingStorage) {
			if (storage == 'setting') {
				return SettingService.getGlobal({ key })
			} else if (storage == 'smtp') {
				return serverConfig.smtp?.[key]
			} else if (storage == 'config') {
				return serverConfig[key]
			}
		}
		values = Object.fromEntries(
			(
				await Promise.all(
					Object.entries(settings).map(
						async ([_, y]) =>
							await Promise.all(y.map(async (x) => [x.key, await getValue(x.key, x.storage)]))
					)
				)
			).flat()
		)
	}

	async function saveSettings() {
		const updateConfig = ConfigService.updateConfig({ name: 'server', requestBody: serverConfig })
		const updateSettings = Promise.all(
			Object.values(settings)
				.flatMap((x) => Object.entries(x))
				.filter((x) => x[1].storage == 'setting')
				.map(async ([_, x]) => {
					await SettingService.updateGlobal({ key: x.key, requestBody: { value: values[x.key] } })
				})
		)
	}

	let oauths: Record<string, any> = {}

	let resourceName = ''
</script>

<CenteredModal title="Instance Settings">
	<div class="flex pb-6">
		<Button variant="border" size="sm" href="/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
	</div>
	<div class="flex-col flex gap-10">
		{#each Object.keys(settings) as category}
			<div>
				<h2 class="pb-2">{category}</h2>
				<div class="flex-col flex gap-1 pb-4">
					{#each settings[category] as setting}
						<label class="block pb-2">
							<span class="text-primary font-semibold text-sm">{setting.label}</span>
							{#if setting.tooltip}
								<Tooltip>{setting.tooltip}</Tooltip>
							{/if}
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
								{:else if setting.fieldType == 'boolean'}
									<div>
										<Toggle bind:checked={values[setting.key]} />
									</div>
								{:else if setting.fieldType == 'seconds'}
									<div>
										<SecondsInput bind:seconds={values[setting.key]} />
									</div>
								{/if}
							{:else}
								<input disabled placeholder="Loading..." />
							{/if}
						</label>
					{/each}
				</div>
			</div>
		{/each}
		<div>
			<h2 class="pb-4">OAuth</h2>

			<h4 class="pb-4">Logins</h4>
			<div class="text-xs italic text-tertiary pb-4">
				Without EE, the number of SSO users is limited to 50. SCIM/SAML is available on EE</div
			>
			<div class="flex flex-col gap-2 pb-4">
				<OAuthSetting name="Google" bind:value={oauths['google']} />
				<OAuthSetting name="Microsoft" bind:value={oauths['microsoft']} />
				<OAuthSetting name="Github" bind:value={oauths['github']} />
				<OAuthSetting name="Gitlab" bind:value={oauths['gitlab']} />
				<OAuthSetting name="Jumpcloud" bind:value={oauths['jumpcloud']} />
				<OAuthSetting name="Okta" bind:value={oauths['okta']} />
				<OAuthSetting name="Keycloak" bind:value={oauths['keycloak']} />
			</div>
			<h4 class="pb-4">Resources</h4>
			{#each Object.keys(oauths) as k}
				{#if !['google', 'microsoft', 'github', 'gitlab', 'jumpcloud', 'okta', 'keycloak'].includes(k)}
					{#if oauths[k]}
						<div class="flex flex-col gap-2 pb-4">
							<label class="text-sm font-medium text-gray-700">{k}</label>
							<label class="block pb-2">
								<span class="text-primary font-semibold text-sm">Client Id</span>
								<input type="text" placeholder="Client Id" bind:value={oauths[k]['id']} />
							</label>
							<label class="block pb-2">
								<span class="text-primary font-semibold text-sm">Client Secret</span>
								<input type="text" placeholder="Client Secret" bind:value={oauths[k]['secret']} />
							</label>
						</div>
					{/if}
				{/if}
			{/each}
			<div class="flex gap-2">
				<input type="text" placeholder="slack" bind:value={resourceName} />
				<Button
					variant="border"
					color="blue"
					hover="yo"
					size="sm"
					endIcon={{ icon: faPlus }}
					disabled={resourceName == ''}
					on:click={() => {
						oauths[resourceName] = { id: '', secret: '' }
						resourceName = ''
					}}
				>
					Add OAuth client for {resourceName}
				</Button>
			</div>
		</div>
	</div>
	<div class="py-4" />
	<pre class="text-xs"
		>{JSON.stringify({ oauths, values }, null, 2)}
	</pre>
	<Button
		on:click={async () => {
			await saveSettings()
			sendUserToast('Settings updated')
		}}>Save</Button
	>
</CenteredModal>
