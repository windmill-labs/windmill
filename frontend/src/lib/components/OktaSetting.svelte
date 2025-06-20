<script lang="ts">
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	interface Props {
		value: any
	}

	let { value = $bindable() }: Props = $props()

	function changeDomain(domain, custom) {
		if (value) {
			let baseUrl = custom ? `https://${domain}` : `https://${domain}.okta.com`
			value = {
				...value,
				login_config: {
					auth_url: `${baseUrl}/oauth2/v1/authorize`,
					token_url: `${baseUrl}/oauth2/v1/token`,
					userinfo_url: `${baseUrl}/oauth2/v1/userinfo`,
					scopes: ['openid', 'profile', 'email']
				},
				connect_config: {
					auth_url: `${baseUrl}/oauth2/v1/authorize`,
					token_url: `${baseUrl}/oauth2/v1/token`,
					scopes: ['openid', 'profile', 'email']
				}
			}
		}
	}
	let enabled = $derived(value != undefined)
	$effect(() => {
		changeDomain(value?.['domain'], value?.['custom'])
	})
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="text-sm font-medium text-primary flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name="okta" after={true} /></div><Toggle
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
						>
							{#snippet children({ item })}
								<ToggleButton value="org" label={'Org'} {item} />
								<ToggleButton value="custom" label="Custom" {item} />
							{/snippet}
						</ToggleButtonGroup>
					</div>
					<div class="grow">
						<span class="text-primary font-semibold text-sm"
							>{#if value['custom']}Custom ({'https://<domain>'}){:else}
								Org ({'https://<your org>.okta.com'}){/if}</span
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
					From your Admin page, setup windmill using the service flow <br />Create a new app
					integration <br />a. For "sign-in method" select "OIDC - Open ID Connect" <br />
					b. For "application type" select "Web Appliction" <br />
					Select all of the following options for Grant type of "Client acting on behalf of a user":
					<br /> Authorization Code Refresh Token Implicit (hybrid) <br />
					Allow ID Token with implicit grant type <br />
					Allow Access Token with implicit grant type <br />
					For Refresh Token, select "Rotate token after every use" <br />
					Under "LOGIN", set the following: <br />"Sign-in redirect URIs"
					`BASE_URL/user/login_callback/okta`<br />
					"Sign-out redirect URIs" `BASE_URL/auth/logout` <br />"Login initiated by" App Only <br />
					"Initiate login URI" `BASE_URL/user/login`
				</div>
			</CollapseLink>
		</div>
	{/if}
</div>
