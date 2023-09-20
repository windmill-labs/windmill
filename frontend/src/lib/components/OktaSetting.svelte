<script lang="ts">
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'

	export let value: any

	$: enabled = value != undefined

	let org = ''

	$: changeOrg(org)

	function changeOrg(org) {
		value = {
			...value,
			login_config: {
				auth_url: `https://${org}.okta.com/oauth2/v1/authorize`,
				token_url: `https://${org}.okta.com/oauth2/v1/token`,
				userinfo_url: `https://${org}.okta.com/oauth2/v1/userinfo`,
				scopes: ['openid', 'profile', 'email']
			},
			connect_config: {
				auth_url: `https://${org}.okta.com/oauth2/v1/authorize`,
				token_url: `https://${org}.okta.com/oauth2/v1/token`,
				scopes: ['openid', 'profile', 'email']
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<label class="text-sm font-medium text-gray-700 flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name="okta" after={true} /></div><Toggle
			checked={enabled}
			on:change={(e) => {
				if (e.detail) {
					value = { id: '', secret: '' }
				} else {
					value = undefined
				}
			}}
		/></label
	>
	{#if enabled}
		<div class="p-2 rounded border">
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Org ({'https://<your org>.okta.com'})</span
				>
				<input type="text" placeholder="yourorg" bind:value={org} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm"
					>Client Id <Tooltip
						>Client credential from the client ID section of the okta service configuration</Tooltip
					></span
				>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm"
					>Client Secret <Tooltip
						>from the CLIENT SECRETS section of the okta service configuration</Tooltip
					></span
				>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
			<CollapseLink text="Instructions">
				<div class="text-sm text-secondary border p-2">
					From your Admin page, setup windmill using the service flow Create a new app integration
					a. For "sign-in method" select "OIDC - Open ID Connect" b. For "application type" select
					"Web Appliction" Select all of the following options for Grant type of "Client acting on
					behalf of a user" Authorization Code Refresh Token Implicit (hybrid) Allow ID Token with
					implicit grant type Allow Access Token with implicit grant type For Refresh Token, select
					"Rotate token after every use" Under "LOGIN", set the following: "Sign-in redirect URIs"
					BASE_URL/user/login_callback/okta "Sign-out redirect URIs" BASE_URL/auth/logout "Login
					initiated by" App Only "Initiate login URI" BASE_URL/user/login
				</div>
			</CollapseLink>
		</div>
	{/if}
</div>
