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
	import Password from './Password.svelte'
	import { capitalize, type Item } from '$lib/utils'
	import ClipboardPanel from './details/ClipboardPanel.svelte'
	import Toggle from './Toggle.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import { APP_TO_ICON_COMPONENT } from './icons'
	import { ExternalLink, Plus, Circle, X } from 'lucide-svelte'
	import AzureOauthSettings from './AzureOauthSettings.svelte'
	import Tooltip from './Tooltip.svelte'
	import { tick } from 'svelte'
	import { Popover } from './meltComponents'
	import SettingsPageHeader from './settings/SettingsPageHeader.svelte'
	import oauthConnectRegistry from '$oauth_connect_registry'

	interface Props {
		// Per-instance OAuth providers (Snowflake, ServiceNow, …): instance name
		// keyed by provider, used to build their per-instance connect_config URLs.
		// Required (and always bound by InstanceSettings) so it is never undefined.
		instanceInputs: Record<string, string>
		oauths?: Record<string, any>
		requirePreexistingUserForOauth?: boolean
		baseUrl?: string
		scim?: import('svelte').Snippet
		tab?: 'sso' | 'oauth' | 'scim'
		hideTabs?: boolean
	}

	let {
		instanceInputs = $bindable(),
		oauths = $bindable(),
		requirePreexistingUserForOauth = $bindable(),
		baseUrl,
		scim,
		tab = $bindable('sso'),
		hideTabs = false
	}: Props = $props()

	$effect(() => {
		if (oauths == undefined) {
			oauths = {}
		}
		if (requirePreexistingUserForOauth == undefined) {
			requirePreexistingUserForOauth = false
		}
	})

	const windmillBuiltinsBase = [
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
		'teams',
		'zoho',
		'xero',
		'apify',
		'docusign',
		'salesforce',
		'outreach'
	]
	// Providers whose registry entry (`backend/oauth_connect.json`) carries a
	// `sandbox` URL block. Each one gets a sibling `<name>_sandbox` dropdown
	// entry and is treated as a builtin so we don't render the custom-URL form
	// — the URLs come from the registry sandbox block. Derived at build time
	// from the registry so adding a sandbox to a provider needs no frontend
	// change.
	const windmillBuiltinsWithSandbox = Object.entries(oauthConnectRegistry)
		.filter(([, cfg]) => cfg && typeof cfg === 'object' && 'sandbox' in cfg)
		.map(([name]) => name)
	// Per-instance providers (Snowflake, ServiceNow, …): registry entries that
	// carry a `connect_config_template`. Derived from the registry so adding a
	// new one needs only a JSON entry — they get a builtin tile + the generic
	// instance-name input below, with no frontend change.
	// Every per-instance templated provider gets a settings tile + instance input:
	// authorization-code ones (ServiceNow) provide an `auth_url`, client-credentials-only
	// ones (Coupa) provide only a `token_url`. The admin enters their instance host so
	// the shared credentials point at the right endpoint.
	const connectConfigTemplates: Record<string, any> = Object.fromEntries(
		Object.entries(oauthConnectRegistry)
			.filter(([, cfg]) => cfg && typeof cfg === 'object' && 'connect_config_template' in cfg)
			.map(([name, cfg]) => [name, (cfg as any).connect_config_template])
	)
	const windmillBuiltinsTemplated = Object.keys(connectConfigTemplates)
	const windmillBuiltins = [
		...windmillBuiltinsBase,
		...windmillBuiltinsWithSandbox.map((n) => `${n}_sandbox`),
		...windmillBuiltinsTemplated
	]

	/** Resolve a `<name>_sandbox` key to its parent registry entry (sandbox
	 * variants inherit the parent's grant_types), matching the connect dialog. */
	function canonicalRegistryKey(name: string): string {
		return name.endsWith('_sandbox') ? name.slice(0, -'_sandbox'.length) : name
	}

	/** The static registry declares client credentials for this provider */
	function registryCcCapable(name: string): boolean {
		return (
			(oauthConnectRegistry as Record<string, any>)[
				canonicalRegistryKey(name)
			]?.grant_types?.includes('client_credentials') ?? false
		)
	}

	/** The static registry supports authorization code for this provider. A
	 * provider with no explicit grant_types defaults to authorization code. */
	function registryAuthCodeCapable(name: string): boolean {
		const reg = (oauthConnectRegistry as Record<string, any>)[canonicalRegistryKey(name)]
		if (!reg) return false
		return reg.grant_types ? reg.grant_types.includes('authorization_code') : true
	}

	/** Built-in provider that only supports client credentials (e.g. Coupa): no
	 * authorization-code flow to choose, so the grant is fixed. */
	function registryCcOnly(name: string): boolean {
		return registryCcCapable(name) && !registryAuthCodeCapable(name)
	}

	/** Map the entry's grant_types to the single-select choice (so the segmented
	 * control always has exactly one selected and can never be empty) */
	function grantChoice(name: string): string {
		const gts = oauths?.[name]?.['grant_types'] ?? ['authorization_code']
		const cc = gts.includes('client_credentials')
		const ac = gts.includes('authorization_code')
		if (cc && ac) return 'both'
		if (cc) return 'client_credentials'
		return 'authorization_code'
	}

	/** Set the grant types from the segmented choice. The instance credentials are
	 * then used for every selected grant — authorization-code popup and/or
	 * server-to-server. */
	function setGrantChoice(name: string, choice: string) {
		if (!oauths || !oauths[name]) return
		oauths[name]['grant_types'] =
			choice === 'both' ? ['authorization_code', 'client_credentials'] : [choice]
	}

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
			newOauths[name] = {
				id: '',
				secret: '',
				grant_types: registryCcOnly(name) ? ['client_credentials'] : ['authorization_code']
			}
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
		// Sandbox variants share the parent provider's icon.
		const lookup = name.endsWith('_sandbox') ? name.slice(0, -'_sandbox'.length) : name

		// Handle special cases
		if (lookup === 'teams') {
			return APP_TO_ICON_COMPONENT.ms_teams_webhook
		}
		if (lookup === 'snowflake_oauth') {
			return APP_TO_ICON_COMPONENT.snowflake
		}
		if (lookup === 'azure_oauth') {
			return APP_TO_ICON_COMPONENT.azure
		}

		// Try direct mapping, fallback to Circle icon if not found
		return APP_TO_ICON_COMPONENT[lookup as keyof typeof APP_TO_ICON_COMPONENT] || Circle
	}

	function generateOAuthDropdownItems(): Item[] {
		const items: Item[] = []

		// Add built-in providers that are not already configured
		windmillBuiltinsBase.forEach((name) => {
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

		// Add sandbox variants for providers that have sandbox URLs in the registry
		windmillBuiltinsWithSandbox.forEach((name) => {
			const sandboxKey = `${name}_sandbox`
			if (!oauths || !oauths[sandboxKey]) {
				const icon = getOAuthProviderIcon(name)
				items.push({
					displayName: `${capitalize(name)} (sandbox)`,
					action: () => createOAuthClient(sandboxKey),
					icon: icon
				})
			}
		})

		// Add per-instance providers (registry entries with a connect_config_template)
		windmillBuiltinsTemplated.forEach((name) => {
			if (!oauths || !oauths[name]) {
				const icon = getOAuthProviderIcon(name)
				items.push({
					// Prefer the template's display_name (properly cased, e.g. "ServiceNow")
					// over capitalize(name) which yields "Servicenow"/"Snowflake_oauth".
					displayName: connectConfigTemplates[name]?.display_name ?? capitalize(name),
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
									<label for="{k}_client_secret_sso" class="block pb-6">
										<span class="text-primary font-semibold text-xs">Client Secret</span>
										<Password
											id="{k}_client_secret_sso"
											placeholder="Client Secret"
											bind:password={oauths[k]['secret']}
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
						{@const headerLabel = k.endsWith('_sandbox')
							? `${k.slice(0, -'_sandbox'.length)} (sandbox)`
							: k}
						<div class="flex flex-col gap-2 pb-6">
							<div class="flex flex-row items-center gap-2">
								<IconComponent size={24} width="24" height="24" class="shrink-0" />
								<!-- svelte-ignore a11y_label_has_associated_control -->
								<label class="text-xs font-semibold text-emphasis">{headerLabel}</label>
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
								<label for="{k}_client_secret_oauth">
									<span class="text-primary font-semibold text-xs">Client Secret</span>
									<Password
										id="{k}_client_secret_oauth"
										placeholder="Client Secret"
										bind:password={oauths[k]['secret']}
									/>
								</label>
								<div class="flex flex-col gap-2 mb-2">
									<span class="text-xs font-semibold text-emphasis">These credentials are for</span>
									{#if !windmillBuiltins.includes(k) || (registryCcCapable(k) && registryAuthCodeCapable(k))}
										<ToggleButtonGroup
											selected={grantChoice(k)}
											onSelected={(v) => setGrantChoice(k, v)}
										>
											{#snippet children({ item })}
												<ToggleButton
													value="authorization_code"
													label="Authorization code"
													showTooltipIcon
													tooltip="Users sign in through a browser popup using this app's Client ID and Secret."
													{item}
												/>
												<ToggleButton
													value="client_credentials"
													label="Client credentials"
													showTooltipIcon
													tooltip={`Server-to-server. Fill Client ID and Secret to share one service account for every connection, or leave them empty so each user brings their own.${!windmillBuiltins.includes(k) ? ' A Token URL is required below.' : ''}`}
													{item}
												/>
												<ToggleButton
													value="both"
													label="Both"
													showTooltipIcon
													tooltip="Offer both flows; the same Client ID and Secret are used for each selected grant."
													{item}
												/>
											{/snippet}
										</ToggleButtonGroup>
									{:else if registryCcCapable(k)}
										<span class="text-xs text-secondary font-normal flex items-center gap-1">
											Client credentials (server-to-server)
											<Tooltip
												>Fill Client ID and Secret to share one service account, or leave them empty
												so each user brings their own.</Tooltip
											>
										</span>
									{:else}
										<span class="text-xs text-secondary font-normal"
											>Authorization code (browser sign-in)</span
										>
									{/if}
								</div>
								{#if k === 'azure_oauth'}
									<AzureOauthSettings bind:connect_config={oauths[k]['connect_config']} />
								{:else if !windmillBuiltins.includes(k) && k != 'slack'}
									<CustomOauth bind:connect_config={oauths[k]['connect_config']} />
								{/if}
								{#if connectConfigTemplates[k]}
									{@const tmpl = connectConfigTemplates[k]}
									<label class="block pb-2">
										<span class="text-primary font-semibold text-xs flex gap-2 items-center">
											{#if tmpl.help_url}
												<a href={tmpl.help_url} target="_blank">{tmpl.label}</a><ExternalLink
													size={12}
												/>
											{:else}
												{tmpl.label}
											{/if}
										</span>
										<input
											type="text"
											placeholder={tmpl.placeholder}
											required={true}
											bind:value={instanceInputs[k]}
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
