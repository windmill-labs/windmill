<script lang="ts">
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	export let value: any

	$: enabled = value != undefined

	$: changeDomain(value?.['domain'], value?.['custom'])

	function changeDomain(domain, custom) {
		if (value) {
			let baseUrl = custom ? `https://${domain}` : `https://${domain}.auth0.com`
			value = {
				...value,
				login_config: {
					auth_url: `${baseUrl}/authorize`,
					token_url: `${baseUrl}/oauth/token`,
					userinfo_url: `${baseUrl}/userinfo`,
					scopes: ['openid', 'profile', 'email']
				},
				connect_config: {
					auth_url: `${baseUrl}/authorize`,
					token_url: `${baseUrl}/oauth/token`,
					scopes: ['openid', 'profile', 'email']
				}
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="text-sm font-medium text-primary flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name="auth0" after={true} /></div><Toggle
			checked={enabled}
			on:change={(e) => {
				if (e.detail) {
					value = { id: '', secret: '', domain: '', custom: false }
				} else {
					value = undefined
				}
			}}
		/></label
	>
	{#if enabled}
		<div class="p-2 rounded border">
			<label class="block pb-2">
				<div class="flex gap-2 items-end">
					<div>
						<ToggleButtonGroup
							selected={value['custom'] ? 'custom' : 'org'}
							on:selected={({ detail }) => {
								value['custom'] = detail === 'custom'
							}}
							let:item
						>
							<ToggleButton value="org" label={'Org'} {item} />
							<ToggleButton value="custom" label="Custom" {item} />
						</ToggleButtonGroup>
					</div>
					<div class="grow">
						<span class="text-primary font-semibold text-sm"
							>{#if value['custom']}Custom ({'https://<domain>'}){:else}
								Org ({'https://<your org>.auth0.com'}){/if}</span
						>
						<input type="text" placeholder="yourorg" bind:value={value['domain']} />
					</div>
				</div>
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Custom Name</span>
				<input type="text" placeholder="Custom Name" bind:value={value['display_name']} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm"
					>Client ID <Tooltip>Client ID credential of the auth0 service configuration</Tooltip
					></span
				>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm"
					>Client Secret <Tooltip>Client Secret of the auth0 service configuration</Tooltip></span
				>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
			<CollapseLink text="Instructions">
				<div class="text-sm text-secondary border p-2">
					From your Admin page, setup a Windmill application<br />
					Create a new application<br />
					For "application type" select "Regular Web Application"<br />
					Copy down the "Client ID" and "Client Secret" and paste them into the fields above <br />
					Under "Application URIs", set the following:<br />
					a. Application Login URI: `BASE_URL/user/login`<br />
					b. Allowed Callback URLs: `BASE_URL/user/login_callback/auth0`<br />
					c. Allowed Logout URLs: `BASE_URL/auth/logout`<br />
					d. Allowed Web Origins: `BASE_URL`<br />
					e. Allowed Origins (CORS): `BASE_URL`<br />
				</div>
			</CollapseLink>
		</div>
	{/if}
</div>
