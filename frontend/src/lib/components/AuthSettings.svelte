<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { Alert, Button, Tab, Tabs } from './common'

	import OAuthSetting from '$lib/components/OAuthSetting.svelte'
	import OktaSetting from './OktaSetting.svelte'
	import Auth0Setting from './Auth0Setting.svelte'
	import KeycloakSetting from './KeycloakSetting.svelte'
	import CustomSso from './CustomSso.svelte'
	import AuthentikSetting from '$lib/components/AuthentikSetting.svelte'
	import AutheliaSetting from '$lib/components/AutheliaSetting.svelte'
	import PocketIdSetting from '$lib/components/PocketIdSetting.svelte'
	import KanidmSetting from '$lib/components/KanidmSetting.svelte'
	import ZitadelSetting from '$lib/components/ZitadelSetting.svelte'
	import NextcloudSetting from '$lib/components/NextcloudSetting.svelte'
	import CustomOauth from './CustomOauth.svelte'
	import { capitalize, type Item } from '$lib/utils'
	import ClipboardPanel from './details/ClipboardPanel.svelte'
	import Toggle from './Toggle.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import { APP_TO_ICON_COMPONENT } from './icons'
	import { ExternalLink, Plus, Circle, X } from 'lucide-svelte'
	import AzureOauthSettings from './AzureOauthSettings.svelte'
	import Tooltip from './Tooltip.svelte'
	import { tick } from 'svelte'
	import { Popover } from './meltComponents'
	import SettingsPageHeader from './settings/SettingsPageHeader.svelte'

	interface Props {
		snowflakeAccountIdentifier?: string
		oauths?: Record<string, any>
		requirePreexistingUserForOauth?: boolean
		baseUrl?: string
		scim?: import('svelte').Snippet
		tab?: 'sso' | 'oauth' | 'scim'
		hideTabs?: boolean
	}

	let {
		snowflakeAccountIdentifier = $bindable(),
		oauths = $bindable(),
		requirePreexistingUserForOauth = $bindable(),
		baseUrl,
		scim,
		tab = $bindable('sso'),
		hideTabs = false
	}: Props = $props()

	$effect(() => {
		if (snowflakeAccountIdentifier == undefined) {
			snowflakeAccountIdentifier = ''
		}
		if (oauths == undefined) {
			oauths = {}
		}
		if (requirePreexistingUserForOauth == undefined) {
			requirePreexistingUserForOauth = false
		}
	})

	const windmillBuiltins = [
		'azure_oauth',
		'github',
		'gitlab',
		'bitbucket',
		'slack',
		'gsheets',
		'gdrive',
		'gmail',
		'gcal',
		'gforms',
		'gcloud',
		'gworkspace',
		'basecamp',
		'linkedin',
		'quickbooks',
		'visma',
		'sage_intacct',
		'spotify',
		'snowflake_oauth',
		'teams',
		'zoho',
		'xero',
		'apify'
	]

	let showCustomOAuthForm = $state(false)
	let customOAuthName = $state('')
	let customNameInput = $state<HTMLInputElement>()
	let dropdownOpen = $state(false)

	let ssoPopoverOpen = $state(false)
	let ssoClientName = $state('')
	let ssoNameInput = $state<HTMLInputElement>()

	function createOAuthClient(name: string) {
		if (oauths && name) {
			// Create a new object to ensure the new item is added at the end
			const newOauths = { ...oauths }
			newOauths[name] = { id: '', secret: '', grant_types: ['authorization_code'] }
			oauths = newOauths
			dropdownOpen = false
		}
	}

	function handleCustomOAuthClient() {
		showCustomOAuthForm = true
		customOAuthName = ''
		dropdownOpen = false
		// Focus the input on next tick
		tick().then(() => {
			customNameInput?.focus()
		})
	}

	function submitCustomOAuthClient() {
		const trimmedName = customOAuthName.trim()
		if (trimmedName) {
			createOAuthClient(trimmedName)
			showCustomOAuthForm = false
			customOAuthName = ''
		}
	}

	function cancelCustomOAuthForm() {
		showCustomOAuthForm = false
		customOAuthName = ''
	}

	function handleCustomOAuthKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault()
			submitCustomOAuthClient()
		} else if (event.key === 'Escape') {
			event.preventDefault()
			cancelCustomOAuthForm()
		}
	}

	function handleSsoPopoverOpen() {
		ssoPopoverOpen = true
		ssoClientName = ''
		// Focus the input on next tick
		tick().then(() => {
			ssoNameInput?.focus()
		})
	}

	function submitSsoClient() {
		const trimmedName = ssoClientName.trim()
		if (trimmedName && oauths) {
			// Create a new object to ensure the new item is added at the end
			const newOauths = { ...oauths }
			newOauths[trimmedName] = { id: '', secret: '', login_config: {} }
			oauths = newOauths
			ssoPopoverOpen = false
			ssoClientName = ''
		}
	}

	function cancelSsoPopover() {
		ssoPopoverOpen = false
		ssoClientName = ''
	}

	function handleSsoKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault()
			submitSsoClient()
		} else if (event.key === 'Escape') {
			event.preventDefault()
			cancelSsoPopover()
		}
	}

	function getOAuthProviderIcon(name: string) {
		// Handle special cases
		if (name === 'teams') {
			return APP_TO_ICON_COMPONENT.ms_teams_webhook
		}
		if (name === 'snowflake_oauth') {
			return APP_TO_ICON_COMPONENT.snowflake
		}
		if (name === 'azure_oauth') {
			return APP_TO_ICON_COMPONENT.azure
		}

		// Try direct mapping, fallback to Circle icon if not found
		return APP_TO_ICON_COMPONENT[name as keyof typeof APP_TO_ICON_COMPONENT] || Circle
	}

	function generateOAuthDropdownItems(): Item[] {
		const items: Item[] = []

		// Add built-in providers that are not already configured
		windmillBuiltins.forEach((name) => {
			// Only show providers that are not already in the oauths object
			if (!oauths || !oauths[name]) {
				const icon = getOAuthProviderIcon(name)
				items.push({
					displayName: capitalize(name),
					action: () => createOAuthClient(name),
					icon: icon
				})
			}
		})

		// Add custom option
		items.push({
			displayName: `Custom OAuth client ${!$enterpriseLicense ? '(requires ee)' : ''}`,
			action: handleCustomOAuthClient,
			disabled: !$enterpriseLicense
		})

		return items
	}
</script>

{#if !hideTabs}
	<div>
		<Tabs bind:selected={tab} class="mb-4">
			<Tab value="sso" label="SSO" />
			<Tab value="oauth" label="OAuth" />
			<Tab value="scim" label="SCIM/SAML" />
		</Tabs>
	</div>
{/if}

<div class="mb-6">
	{#if oauths}
		{#if tab === 'sso'}
			<SettingsPageHeader
				title="Single Sign-On"
				description="Configure SSO providers to let users authenticate using their existing identity provider credentials. To test SSO, save the settings and try to login in an incognito window."
				link="https://www.windmill.dev/docs/misc/setup_oauth#sso"
			/>
			{#if !$enterpriseLicense || $enterpriseLicense.endsWith('_pro')}
				<Alert type="info" title="Limited to 10 SSO users">
					Without EE, the number of SSO users is limited to 10. SCIM/SAML is available on EE
				</Alert>
				<div class="mb-2"></div>
			{/if}
			<div class="flex gap-2 py-4">
				<Toggle
					options={{
						right: 'Require users to have been added manually to Windmill to sign in through SSO'
					}}
					bind:checked={requirePreexistingUserForOauth}
				/>
			</div>
			<div class="flex flex-col gap-4 py-4">
				<OAuthSetting name="google" bind:value={oauths['google']} />
				<OAuthSetting name="microsoft" bind:value={oauths['microsoft']} />
				<OktaSetting bind:value={oauths['okta']} />
				<Auth0Setting bind:value={oauths['auth0']} />
				<OAuthSetting name="github" bind:value={oauths['github']} />
				<OAuthSetting name="gitlab" bind:value={oauths['gitlab']} />
				<OAuthSetting name="jumpcloud" bind:value={oauths['jumpcloud']} />
				<KeycloakSetting bind:value={oauths['keycloak']} />
				<AuthentikSetting bind:value={oauths['authentik']} />
				<AutheliaSetting bind:value={oauths['authelia']} />
				<PocketIdSetting bind:value={oauths['pocketid']} />
				<KanidmSetting bind:value={oauths['kanidm']} />
				<ZitadelSetting bind:value={oauths['zitadel']} />
				<NextcloudSetting bind:value={oauths['nextcloud']} {baseUrl} />
				{#each Object.keys(oauths) as k}
					{#if !['authelia', 'authentik', 'google', 'microsoft', 'github', 'gitlab', 'jumpcloud', 'okta', 'auth0', 'keycloak', 'slack', 'kanidm', 'zitadel', 'nextcloud', 'pocketid'].includes(k) && oauths[k] && 'login_config' in oauths[k]}
						{#if oauths[k]}
							<div class="flex flex-col gap-2 pb-4">
								<div class="flex flex-row items-center gap-2">
									<!-- svelte-ignore a11y_label_has_associated_control -->
									<label class="text-xs font-semibold text-emphasis">{k}</label>
									<Button
										variant="subtle"
										destructive
										iconOnly
										unifiedSize="sm"
										startIcon={{ icon: X }}
										onclick={() => {
											if (oauths) {
												delete oauths[k]
												oauths = { ...oauths }
											}
										}}
									/>
								</div>
								<div class="p-4 rounded bg-surface-tertiary shadow-sm">
									<label class="block pb-6">
										<span class="text-primary font-semibold text-xs">Custom Name</span>
										<input
											type="text"
											placeholder="Custom Name"
											bind:value={oauths[k]['display_name']}
										/>
									</label>
									<label class="block pb-6">
										<span class="text-primary font-semibold text-xs">Client Id</span>
										<input type="text" placeholder="Client Id" bind:value={oauths[k]['id']} />
									</label>
									<label class="block pb-6">
										<span class="text-primary font-semibold text-xs">Client Secret</span>
										<input
											type="text"
											placeholder="Client Secret"
											bind:value={oauths[k]['secret']}
										/>
									</label>
									{#if !windmillBuiltins.includes(k) && k != 'slack'}
										<CustomSso bind:login_config={oauths[k]['login_config']} />
									{/if}
								</div>
							</div>
						{/if}
					{/if}
				{/each}
			</div>
			<div class="flex justify-start py-4">
				<Popover placement="bottom-start" bind:isOpen={ssoPopoverOpen} onClose={cancelSsoPopover}>
					{#snippet trigger()}
						<Button
							variant="default"
							hover="yo"
							size="sm"
							endIcon={{ icon: Plus }}
							disabled={!$enterpriseLicense}
							onclick={handleSsoPopoverOpen}
						>
							Add custom SSO client {!$enterpriseLicense ? '(requires ee)' : ''}
						</Button>
					{/snippet}
					{#snippet content()}
						<div class="flex flex-col gap-2 p-4 min-w-64">
							<div class="flex gap-2">
								<input
									type="text"
									placeholder="Custom SSO client name"
									bind:value={ssoClientName}
									onkeydown={handleSsoKeydown}
									class="flex-1 px-3 py-2 text-sm border rounded"
									bind:this={ssoNameInput}
								/>
								<Button
									size="sm"
									variant="accent"
									disabled={!ssoClientName.trim()}
									onclick={submitSsoClient}
								>
									Add
								</Button>
								<Button size="sm" variant="subtle" onclick={cancelSsoPopover}>Cancel</Button>
							</div>
						</div>
					{/snippet}
				</Popover>
			</div>
		{:else if tab === 'oauth'}
			<SettingsPageHeader
				title="OAuth"
				description="Connect third-party services like Slack, Teams or Google to let users authenticate directly from Windmill and automatically obtain access tokens."
				link="https://www.windmill.dev/docs/misc/setup_oauth#oauth"
			/>
			<div class="h-1"></div>
			<OAuthSetting login={false} name="slack" bind:value={oauths['slack']} />
			<div class="h-6"></div>
			<OAuthSetting login={false} name="teams" eeOnly={true} bind:value={oauths['teams']} />
			<div class="h-6"></div>

			{#each Object.keys(oauths) as k}
				{#if oauths[k] && !(oauths[k] && 'login_config' in oauths[k])}
					{#if !['slack', 'teams'].includes(k) && oauths[k]}
						{@const IconComponent = getOAuthProviderIcon(k) as any}
						<div class="flex flex-col gap-2 pb-6">
							<div class="flex flex-row items-center gap-2">
								<IconComponent size={24} width="24" height="24" class="shrink-0" />
								<!-- svelte-ignore a11y_label_has_associated_control -->
								<label class="text-xs font-semibold text-emphasis">{k}</label>
								<Button
									variant="subtle"
									destructive
									onclick={() => {
										if (oauths) {
											delete oauths[k]
											oauths = { ...oauths }
										}
									}}
									iconOnly
									unifiedSize="sm"
									startIcon={{ icon: X }}
									wrapperClasses="h-fit w-fit"
								/>
							</div>
							<div class="p-4 border rounded-md flex flex-col gap-6">
								<label>
									<span class="text-primary font-semibold text-xs">Client Id</span>
									<input type="text" placeholder="Client Id" bind:value={oauths[k]['id']} />
								</label>
								<label>
									<span class="text-primary font-semibold text-xs">Client Secret</span>
									<input type="text" placeholder="Client Secret" bind:value={oauths[k]['secret']} />
								</label>
								{#if k === 'visma' || !windmillBuiltins.includes(k)}
									<div class="mb-8">
										<div style="display: flex; align-items: center; gap: 8px;">
											<input
												type="checkbox"
												style="width: 16px; height: 16px; margin: 0;"
												checked={oauths?.[k]?.['grant_types']?.includes('client_credentials') ??
													false}
												onchange={(e) => {
													const target = e.target as HTMLInputElement
													if (oauths && oauths[k]) {
														if (!oauths[k]['grant_types']) {
															oauths[k]['grant_types'] = ['authorization_code']
														}
														if (target.checked) {
															if (!oauths[k]['grant_types'].includes('client_credentials')) {
																oauths[k]['grant_types'] = [
																	...oauths[k]['grant_types'],
																	'client_credentials'
																]
															}
														} else {
															oauths[k]['grant_types'] = oauths[k]['grant_types'].filter(
																(gt: string) => gt !== 'client_credentials'
															)
														}
													}
												}}
											/>
											<span class="text-xs font-semibold text-emphasis"
												>Support Client Credentials Flow</span
											>
											<Tooltip>
												Enables server-to-server authentication without user interaction. Use for
												automated scripts and background jobs.
												<br /><br />
												When enabled, users can provide their own client credentials at the resource
												level. The Client ID and Secret configured above are only used for the traditional
												OAuth flow (popup window).
											</Tooltip>
										</div>
									</div>
								{/if}
								{#if k === 'azure_oauth'}
									<AzureOauthSettings bind:connect_config={oauths[k]['connect_config']} />
								{:else if !windmillBuiltins.includes(k) && k != 'slack'}
									<CustomOauth bind:connect_config={oauths[k]['connect_config']} />
								{/if}
								{#if k == 'snowflake_oauth'}
									<label class="block pb-2">
										<span class="text-primary font-semibold text-xs flex gap-2 items-center"
											><a
												href="https://docs.snowflake.com/en/user-guide/admin-account-identifier#using-an-account-name-as-an-identifier"
												target="_blank">Snowflake Account Identifier</a
											><ExternalLink size={12} /></span
										>
										<input
											type="text"
											placeholder="<orgname>-<account_name>"
											required={true}
											bind:value={snowflakeAccountIdentifier}
										/>
									</label>
								{/if}
								{#if k === 'gworkspace'}
									<div>
										<Toggle
											options={{
												right:
													'Allow workspace admins to setup Google native triggers using these credentials'
											}}
											checked={oauths[k]?.share_with_workspaces ?? false}
											on:change={(e) => {
												if (oauths && oauths[k]) {
													oauths[k] = { ...oauths[k], share_with_workspaces: e.detail }
												}
											}}
										/>
										{#if oauths[k]?.share_with_workspaces}
											<p class="text-xs text-tertiary mt-1">
												Workspace admins will be able to connect Google native triggers without
												configuring their own OAuth client. The credentials are not exposed to them.
											</p>
											<p class="text-xs text-tertiary mt-2">
												Add the following redirect URI to
												<a
													href="https://console.cloud.google.com/apis/credentials"
													target="_blank"
													class="underline">Google Cloud Console</a
												>:
											</p>
											<div class="mt-1">
												<ClipboardPanel
													content="{baseUrl}/workspace_settings?tab=native_triggers&service=google"
													size="sm"
												/>
											</div>
										{/if}
									</div>
								{/if}
							</div>
						</div>
					{/if}
				{/if}
			{/each}

			{#if showCustomOAuthForm}
				<div class="flex flex-col gap-2 p-4 border rounded-lg bg-surface">
					<div class="flex gap-2">
						<input
							type="text"
							placeholder="Custom OAuth client name"
							bind:value={customOAuthName}
							onkeydown={handleCustomOAuthKeydown}
							class="flex-1 px-3 py-2 text-sm border rounded"
							bind:this={customNameInput}
						/>
						<Button
							size="sm"
							variant="accent"
							disabled={!customOAuthName.trim()}
							onclick={submitCustomOAuthClient}
						>
							Add
						</Button>
						<Button size="sm" variant="subtle" onclick={cancelCustomOAuthForm}>Cancel</Button>
					</div>
				</div>
			{:else}
				<div class="flex justify-start">
					<DropdownV2
						placement="bottom-start"
						items={generateOAuthDropdownItems}
						btnText="Add OAuth client"
						maxHeight="25vh"
						customMenu={true}
						bind:open={dropdownOpen}
					>
						{#snippet buttonReplacement()}
							<Button variant="default" hover="yo" size="sm" endIcon={{ icon: Plus }}>
								Add OAuth client
							</Button>
						{/snippet}
						{#snippet menu()}
							<div
								class="bg-surface-tertiary dark:border w-56 origin-top-right rounded-lg shadow-lg focus:outline-none overflow-y-auto py-1"
								style="max-height: 25vh;"
							>
								{#each generateOAuthDropdownItems() as item}
									{@const IconComponent = item.icon}
									<button
										class="w-full px-4 py-2 text-left hover:bg-surface-hover transition-colors flex items-center gap-2"
										class:opacity-50={item.disabled}
										disabled={item.disabled}
										onclick={item.action}
									>
										<IconComponent
											size={14}
											width="14"
											height="14"
											class="shrink-0 w-[14px] h-[14px]"
										/>
										<span class="text-xs font-normal text-primary truncate grow min-w-0">
											{item.displayName}
										</span>
									</button>
								{/each}
							</div>
						{/snippet}
					</DropdownV2>
				</div>
			{/if}
		{:else if tab == 'scim'}
			<SettingsPageHeader
				title="SCIM/SAML"
				description="Set up SAML and SCIM to authenticate users using your identity provider."
				link="https://www.windmill.dev/docs/misc/saml_and_scim"
			/>
			{@render scim?.()}
		{/if}
	{/if}
</div>
