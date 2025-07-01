<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { Alert, Button, Tab, Tabs } from './common'

	import OAuthSetting from '$lib/components/OAuthSetting.svelte'
	import OktaSetting from './OktaSetting.svelte'
	import Auth0Setting from './Auth0Setting.svelte'
	import CloseButton from './common/CloseButton.svelte'
	import KeycloakSetting from './KeycloakSetting.svelte'
	import CustomSso from './CustomSso.svelte'
	import AuthentikSetting from '$lib/components/AuthentikSetting.svelte'
	import AutheliaSetting from '$lib/components/AutheliaSetting.svelte'
	import KanidmSetting from '$lib/components/KanidmSetting.svelte'
	import ZitadelSetting from '$lib/components/ZitadelSetting.svelte'
	import CustomOauth from './CustomOauth.svelte'
	import { capitalize } from '$lib/utils'
	import Toggle from './Toggle.svelte'
	import { ExternalLink, Plus } from 'lucide-svelte'
	import AzureOauthSettings from './AzureOauthSettings.svelte'
	import Tooltip from './Tooltip.svelte'

	interface Props {
		snowflakeAccountIdentifier?: string
		oauths?: Record<string, any>
		requirePreexistingUserForOauth?: boolean
		scim?: import('svelte').Snippet
	}

	let {
		snowflakeAccountIdentifier = $bindable(),
		oauths = $bindable(),
		requirePreexistingUserForOauth = $bindable(),
		scim
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
		'spotify',
		'snowflake_oauth',
		'teams',
		'xero'
	]

	let oauth_name = $state(undefined)

	let clientName = $state('')
	let resourceName = $state('')

	let tab: 'sso' | 'oauth' | 'scim' = $state('sso')
</script>

<div>
	<Tabs bind:selected={tab} class="mt-2 mb-4">
		<Tab value="sso">SSO</Tab>
		<Tab value="oauth">OAuth</Tab>
		<Tab value="scim">SCIM/SAML</Tab>
	</Tabs>
</div>

<div class="mb-6">
	{#if oauths}
		{#if tab === 'sso'}
			{#if !$enterpriseLicense || $enterpriseLicense.endsWith('_pro')}
				<Alert type="warning" title="Limited to 10 SSO users">
					Without EE, the number of SSO users is limited to 10. SCIM/SAML is available on EE
				</Alert>
			{/if}

			<div class="py-1"></div>
			<div class="mb-2">
				<span class="text-primary text-sm"
					>When at least one of the below options is set, users will be able to login to Windmill
					via their third-party account.
					<br /> To test SSO, the recommended workflow is to to save the settings and try to login
					in an incognito window.
					<a target="_blank" href="https://www.windmill.dev/docs/misc/setup_oauth#sso">Learn more</a
					></span
				>
			</div>
			<div class="flex flex-col gap-3 py-4">
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
				<KanidmSetting bind:value={oauths['kanidm']} />
				<ZitadelSetting bind:value={oauths['zitadel']} />
				{#each Object.keys(oauths) as k}
					{#if !['authelia', 'authentik', 'google', 'microsoft', 'github', 'gitlab', 'jumpcloud', 'okta', 'auth0', 'keycloak', 'slack', 'kanidm', 'zitadel'].includes(k) && 'login_config' in oauths[k]}
						{#if oauths[k]}
							<div class="flex flex-col gap-2 pb-4">
								<div class="flex flex-row items-center gap-2">
									<!-- svelte-ignore a11y_label_has_associated_control -->
									<label class="text-md font-medium text-primary">{k}</label>
									<CloseButton
										on:close={() => {
											if (oauths) {
												delete oauths[k]
												oauths = { ...oauths }
											}
										}}
									/>
								</div>
								<div class="p-2 border rounded">
									<label class="block pb-2">
										<span class="text-primary font-semibold text-sm">Custom Name</span>
										<input
											type="text"
											placeholder="Custom Name"
											bind:value={oauths[k]['display_name']}
										/>
									</label>
									<label class="block pb-2">
										<span class="text-primary font-semibold text-sm">Client Id</span>
										<input type="text" placeholder="Client Id" bind:value={oauths[k]['id']} />
									</label>
									<label class="block pb-2">
										<span class="text-primary font-semibold text-sm">Client Secret</span>
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
			<div class="flex gap-2 py-4">
				<input type="text" placeholder="client_id" bind:value={clientName} />
				<Button
					variant="border"
					color="blue"
					hover="yo"
					size="sm"
					endIcon={{ icon: Plus }}
					disabled={clientName == ''}
					on:click={() => {
						if (oauths) {
							oauths[clientName] = { id: '', secret: '', login_config: {} }
						}
						clientName = ''
					}}
				>
					Add custom SSO client {!$enterpriseLicense ? '(requires ee)' : ''}
				</Button>
			</div>
			<div class="flex gap-2 py-4">
				<Toggle
					options={{
						right: 'Require users to have been added manually to Windmill to sign in through OAuth'
					}}
					bind:checked={requirePreexistingUserForOauth}
				/>
			</div>
		{:else if tab === 'oauth'}
			<div class="mb-2">
				<span class="text-primary text-sm"
					>When one of the below options is set, you will be able to create a specific resource
					containing a token automatically generated by the third-party provider.
					<br />
					To test it after setting an oauth client, go to the Resources menu and create a new one of
					the type of your oauth client (i.e. a 'github' resource if you set Github OAuth).
					<br /><a target="_blank" href="https://www.windmill.dev/docs/misc/setup_oauth#oauth"
						>Learn more</a
					></span
				>
			</div>
			<div class="py-1"></div>
			<OAuthSetting login={false} name="slack" bind:value={oauths['slack']} />
			<div class="py-1"></div>
			<OAuthSetting login={false} name="teams" eeOnly={true} bind:value={oauths['teams']} />
			<div class="py-1"></div>

			{#each Object.keys(oauths) as k}
				{#if oauths[k] && !('login_config' in oauths[k])}
					{#if !['slack', 'teams'].includes(k) && oauths[k]}
						<div class="flex flex-col gap-2 pb-4">
							<div class="flex flex-row items-center gap-2">
								<!-- svelte-ignore a11y_label_has_associated_control -->
								<label class="text-md font-medium text-primary">{k}</label>
								<CloseButton
									on:close={() => {
										if (oauths) {
											delete oauths[k]
											oauths = { ...oauths }
										}
									}}
								/>
							</div>
							<div class="p-2 border rounded">
								<label class="block pb-2">
									<span class="text-primary font-semibold text-sm">Client Id</span>
									<input type="text" placeholder="Client Id" bind:value={oauths[k]['id']} />
								</label>
								<label class="block pb-2">
									<span class="text-primary font-semibold text-sm">Client Secret</span>
									<input type="text" placeholder="Client Secret" bind:value={oauths[k]['secret']} />
								</label>
								<div style="margin-bottom: 8px;">
									<div style="display: flex; align-items: center; gap: 8px;">
										<input 
											type="checkbox" 
											style="width: 16px; height: 16px; margin: 0;"
											checked={oauths?.[k]?.['grant_types']?.includes('client_credentials') ?? false}
											onchange={(e) => {
												const target = e.target as HTMLInputElement;
												if (oauths && oauths[k]) {
													if (!oauths[k]['grant_types']) {
														oauths[k]['grant_types'] = ['authorization_code']
													}
													if (target.checked) {
														if (!oauths[k]['grant_types'].includes('client_credentials')) {
															oauths[k]['grant_types'] = [...oauths[k]['grant_types'], 'client_credentials']
														}
													} else {
														oauths[k]['grant_types'] = oauths[k]['grant_types'].filter(gt => gt !== 'client_credentials')
													}
												}
											}}
										/>
										<span style="font-size: 14px; font-weight: 600;">Support Client Credentials Flow</span>
										<Tooltip>
											Enables server-to-server authentication without user interaction. Use for automated scripts and background jobs.
											<br><br>
											When enabled, resources can be created using client credentials instead of requiring user authentication.
										</Tooltip>
									</div>
								</div>
								{#if k === 'azure_oauth'}
									<AzureOauthSettings bind:connect_config={oauths[k]['connect_config']} />
								{:else if !windmillBuiltins.includes(k) && k != 'slack'}
									<CustomOauth bind:connect_config={oauths[k]['connect_config']} />
								{/if}
								{#if k == 'snowflake_oauth'}
									<label class="block pb-2">
										<span class="text-primary font-semibold text-sm flex gap-2 items-center"
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
							</div>
						</div>
					{/if}
				{/if}
			{/each}

			<div class="flex gap-2">
				<select name="oauth_name" id="oauth_name" bind:value={oauth_name}>
					<option value={undefined}>Select an OAuth client</option>
					<option value="custom">Fully Custom (requires ee)</option>
					{#each windmillBuiltins as name}
						<option value={name}>{capitalize(name)}</option>
					{/each}
				</select>
				{#if oauth_name == 'custom'}
					<input type="text" placeholder="client_id" bind:value={resourceName} />
				{:else}
					<input type="text" value={oauth_name ?? ''} disabled />
				{/if}
				<Button
					variant="border"
					color="blue"
					hover="yo"
					size="sm"
					endIcon={{ icon: Plus }}
					disabled={!oauth_name ||
						(oauth_name == 'custom' && resourceName == '') ||
						(oauth_name == 'custom' && !$enterpriseLicense)}
					on:click={() => {
						if (oauths) {
							let name = oauth_name == 'custom' ? resourceName : oauth_name
							oauths[name ?? ''] = { id: '', secret: '', grant_types: ['authorization_code'] }
						}
						resourceName = ''
					}}
				>
					Add OAuth client {oauth_name == 'custom' && !$enterpriseLicense ? '(requires ee)' : ''}
				</Button>
			</div>
		{:else if tab == 'scim'}
			{@render scim?.()}
		{/if}
	{/if}
</div>
