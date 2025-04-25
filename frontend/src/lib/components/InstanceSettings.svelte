<script lang="ts">
	import { scimSamlSetting, settings, settingsKeys, type SettingStorage } from './instanceSettings'
	import { Button, Tab, TabContent, Tabs } from '$lib/components/common'
	import { SettingService, SettingsService } from '$lib/gen'
	import type { TeamInfo, TeamsChannel } from '$lib/gen/types.gen'

	import { sendUserToast } from '$lib/toast'
	import { deepEqual } from 'fast-equals'

	import { sleep } from '$lib/utils'
	import { enterpriseLicense } from '$lib/stores'

	import { createEventDispatcher } from 'svelte'
	import { setLicense } from '$lib/enterpriseUtils'
	import AuthSettings from './AuthSettings.svelte'
	import InstanceSetting from './InstanceSetting.svelte'
	import { writable, type Writable } from 'svelte/store'

	export let tab: string = 'Core'
	export let hideTabs: boolean = false
	export let hideSave: boolean = false
	export let closeDrawer: (() => void) | undefined = () => {}

	let values: Writable<Record<string, any>> = writable({})
	let initialOauths: Record<string, any> = {}
	let initialRequirePreexistingUserForOauth: boolean = false
	let requirePreexistingUserForOauth: boolean = false

	let initialValues: Record<string, any> = {}
	let snowflakeAccountIdentifier = ''
	let version: string = ''
	let loading = true

	loadSettings()
	loadVersion()

	const dispatch = createEventDispatcher()

	async function loadVersion() {
		version = await SettingsService.backendVersion()
	}
	let oauths: Record<string, any> = {}

	async function loadSettings() {
		loading = true

		async function getValue(key: string, storage: SettingStorage) {
			if (storage == 'setting') {
				return SettingService.getGlobal({ key })
			}
		}
		initialOauths = (await SettingService.getGlobal({ key: 'oauths' })) ?? {}
		requirePreexistingUserForOauth =
			((await SettingService.getGlobal({ key: 'require_preexisting_user_for_oauth' })) as any) ??
			false
		initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth
		oauths = JSON.parse(JSON.stringify(initialOauths))
		initialValues = Object.fromEntries(
			(
				await Promise.all(
					[...Object.values(settings), scimSamlSetting].map(
						async (y) =>
							await Promise.all(y.map(async (x) => [x.key, await getValue(x.key, x.storage)]))
					)
				)
			).flat()
		)
		let nvalues = JSON.parse(JSON.stringify(initialValues))
		if (nvalues['base_url'] == undefined) {
			nvalues['base_url'] = window.location.origin
		}
		if (nvalues['retention_period_secs'] == undefined) {
			nvalues['retention_period_secs'] = 60 * 60 * 24 * 30
		}
		if (nvalues['base_url'] == undefined) {
			nvalues['base_url'] = 'http://localhost'
		}
		if (nvalues['smtp_settings'] == undefined) {
			nvalues['smtp_settings'] = {}
		}
		if (nvalues['otel'] == undefined) {
			nvalues['otel'] = {}
		}
		if (nvalues['indexer_settings'] == undefined) {
			nvalues['indexer_settings'] = {}
		}

		if (nvalues['critical_error_channels'] == undefined) {
			nvalues['critical_error_channels'] = []
		} else {
			let teams = ((await SettingService.getGlobal({ key: 'teams' })) as TeamInfo[]) ?? []

			nvalues['teams'] = teams

			nvalues['critical_error_channels'] = nvalues['critical_error_channels'].map((el) => {
				if (el.teams_channel) {
					const team = teams.find((team) => team.team_name === el.teams_channel.team_name) || null
					return {
						teams_channel: {
							team_id: team?.team_id,
							team_name: team?.team_name,
							channel_id: team?.channels.find(
								(channel) => channel.channel_id === el.teams_channel.channel_id
							)?.channel_id,
							channel_name: team?.channels.find(
								(channel) => channel.channel_id === el.teams_channel.channel_id
							)?.channel_name
						}
					}
				}
				return el
			})
		}

		$values = nvalues
		loading = false

		// populate snowflake account identifier from db
		const account_identifier =
			oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier
		if (account_identifier) {
			snowflakeAccountIdentifier = account_identifier
		}
	}

	export async function saveSettings() {
		if (
			oauths?.snowflake_oauth &&
			oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier !==
				snowflakeAccountIdentifier
		) {
			setupSnowflakeUrls()
		}

		// Remove empty or invalid teams_channel entries
		$values.critical_error_channels = $values.critical_error_channels.filter((entry) => {
			if (entry && typeof entry == 'object' && 'teams_channel' in entry) {
				return isValidTeamsChannel(entry.teams_channel)
			}
			return true
		})

		let shouldReloadPage = false
		if ($values) {
			const allSettings = [...Object.values(settings), scimSamlSetting].flatMap((x) =>
				Object.entries(x)
			)
			let licenseKeySet = false
			await Promise.all(
				allSettings
					.filter((x) => {
						return (
							x[1].storage == 'setting' &&
							!deepEqual(initialValues?.[x[1].key], $values?.[x[1].key]) &&
							($values?.[x[1].key] != '' ||
								initialValues?.[x[1].key] != undefined ||
								initialValues?.[x[1].key] != null)
						)
					})
					.map(async ([_, x]) => {
						if (x.key == 'license_key') {
							licenseKeySet = true
						}
						if (x.requiresReloadOnChange) {
							shouldReloadPage = true
						}
						return await SettingService.setGlobal({
							key: x.key,
							requestBody: { value: $values?.[x.key] }
						})
					})
			)
			initialValues = JSON.parse(JSON.stringify($values))

			if (!deepEqual(initialOauths, oauths)) {
				await SettingService.setGlobal({
					key: 'oauths',
					requestBody: {
						value: oauths
					}
				})
				initialOauths = JSON.parse(JSON.stringify(oauths))
			}
			if (initialRequirePreexistingUserForOauth !== requirePreexistingUserForOauth) {
				await SettingService.setGlobal({
					key: 'require_preexisting_user_for_oauth',
					requestBody: { value: requirePreexistingUserForOauth }
				})
			}
			if (licenseKeySet) {
				setLicense()
			}
		} else {
			console.error('Values not loaded')
		}
		if (shouldReloadPage) {
			sendUserToast('Settings updated, reloading page...')
			await sleep(1000)
			window.location.reload()
		} else {
			sendUserToast('Settings updated')
			dispatch('saved')
		}
	}

	function setupSnowflakeUrls() {
		// strip all whitespaces from account identifier
		snowflakeAccountIdentifier = snowflakeAccountIdentifier.replace(/\s/g, '')

		const connect_config = {
			scopes: [],
			auth_url: `https://${snowflakeAccountIdentifier}.snowflakecomputing.com/oauth/authorize`,
			token_url: `https://${snowflakeAccountIdentifier}.snowflakecomputing.com/oauth/token-request`,
			req_body_auth: false,
			extra_params: { account_identifier: snowflakeAccountIdentifier },
			extra_params_callback: {}
		}
		oauths['snowflake_oauth'].connect_config = connect_config
	}

	async function sendStats() {
		await SettingService.sendStats()
		sendUserToast('Usage sent')
	}

	function isValidTeamsChannel(value: any): value is TeamsChannel {
		return (
			typeof value === 'object' &&
			value !== null &&
			typeof value.team_id === 'string' &&
			value.team_id.trim() !== '' &&
			typeof value.team_name === 'string' &&
			value.team_name.trim() !== '' &&
			typeof value.channel_id === 'string' &&
			value.channel_id.trim() !== '' &&
			typeof value.channel_name === 'string' &&
			value.channel_name.trim() !== ''
		)
	}
</script>

<div class="pb-8">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<Tabs {hideTabs} bind:selected={tab}>
		{#each settingsKeys as category}
			<Tab value={category}>{category}</Tab>
		{/each}

		<svelte:fragment slot="content">
			{#each Object.keys(settings) as category}
				<TabContent value={category}>
					{#if category == 'SMTP'}
						<div class="text-secondary pb-4 text-xs"
							>Setting SMTP unlocks sending emails upon adding new users to the workspace or the
							instance or sending critical alerts.
							<a
								target="_blank"
								href="https://www.windmill.dev/docs/advanced/instance_settings#smtp">Learn more</a
							></div
						>
					{:else if category == 'Indexer/Search'}
						<div class="text-secondary pb-4 text-xs"
							>The indexer service unlocks full text search across jobs and service logs. It
							requires spinning up its own separate container
							<a target="_blank" href="https://www.windmill.dev/docs/core_concepts/search_bar#setup"
								>Learn how to</a
							></div
						>
					{:else if category == 'Registries'}
						<div class="text-secondary pb-4 text-xs">
							Add private registries for Pip, Bun and npm. <a
								target="_blank"
								href="https://www.windmill.dev/docs/advanced/imports">Learn more</a
							>
						</div>
					{:else if category == 'Slack'}
						<div class="text-secondary pb-4 text-xs">
							Connecting your instance to a Slack workspace enables critical alerts to be sent to a
							Slack channel.
							<a target="_blank" href="https://www.windmill.dev/docs/misc/saml_and_scim"
								>Learn more</a
							>
						</div>
					{:else if category == 'SCIM/SAML'}
						<div class="text-secondary pb-4 text-xs">
							Setting up SAML and SCIM allows you to authenticate users using your identity
							provider.
							<a
								target="_blank"
								href="https://www.windmill.dev/docs/advanced/instance_settings#slack">Learn more</a
							>
						</div>
					{:else if category == 'Debug'}
						<div class="text-secondary pb-4 text-xs">
							Enable debug mode to get more detailed logs.
						</div>
					{:else if category == 'Telemetry'}
						<div class="text-secondary pb-4 text-xs">
							Anonymous usage data is collected to help improve Windmill.
							<br />The following information is collected:
							<ul class="list-disc list-inside pl-2">
								<li>version of your instances</li>
								<li>instance base URL</li>
								<li>job usage (language, total duration, count)</li>
								<li>login type usage (login type, count)</li>
								<li>worker usage (worker, worker instance, vCPUs, memory)</li>
								<li>user usage (author count, operator count)</li>
								<li>superadmin email addresses</li>
								<li>vCPU usage</li>
								<li>memory usage</li>
								<li>development instance status</li>
							</ul>
						</div>
						{#if $enterpriseLicense}
							<div class="text-secondary pb-4 text-xs">
								On Enterprise Edition, you must send data to check that usage is in line with the
								terms of the subscription. You can either enable telemetry or regularly send usage
								data by clicking the button below.
							</div>
							<Button
								on:click={sendStats}
								variant="border"
								color="light"
								btnClasses="w-auto"
								wrapperClasses="mb-4"
								size="xs">Send usage</Button
							>
						{/if}
					{:else if category == 'Auth/OAuth/SAML'}
						<AuthSettings
							bind:oauths
							bind:snowflakeAccountIdentifier
							bind:requirePreexistingUserForOauth
						>
							<svelte:fragment slot="scim">
								<div class="flex-col flex gap-2 pb-4">
									{#each scimSamlSetting as setting}
										<InstanceSetting
											on:closeDrawer={() => closeDrawer?.()}
											{loading}
											{setting}
											{values}
											{version}
										/>
									{/each}
								</div>
							</svelte:fragment>
						</AuthSettings>
					{/if}
					<div>
						<div class="flex-col flex gap-4 pb-4">
							{#each settings[category] as setting}
								<InstanceSetting
									on:closeDrawer={() => closeDrawer?.()}
									{loading}
									{setting}
									{values}
									{version}
								/>
							{/each}
						</div>
					</div>
				</TabContent>
			{/each}
		</svelte:fragment>
	</Tabs>
</div>

{#if !hideSave}
	<Button on:click={saveSettings}>Save settings</Button>
	<div class="pb-8"></div>
{/if}
