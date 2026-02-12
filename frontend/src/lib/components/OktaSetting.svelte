<script lang="ts">
	import { untrack } from 'svelte'
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface Props {
		value: any
	}

	let { value = $bindable() }: Props = $props()

	let lastValues = { domain: undefined, custom: undefined }

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
	$effect.pre(() => {
		if (value?.['domain'] != lastValues.domain || value?.['custom'] != lastValues.custom) {
			lastValues = { domain: value?.['domain'], custom: value?.['custom'] }
			untrack(() => changeDomain(value?.['domain'], value?.['custom']))
		}
	})
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
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
		<SettingCard class="flex flex-col gap-6">
			<label>
				<div class="flex gap-2 items-start">
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
					<div class="grow flex flex-col gap-1">
						<input type="text" placeholder="yourorg" bind:value={value['domain']} />
						<span class="text-hint font-normal text-2xs"
							>{#if value['custom']}Custom ({'https://<domain>'}){:else}
								Org ({'https://<your org>.okta.com'}){/if}</span
						>
					</div>
				</div>
			</label>
			<label>
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<input type="text" placeholder="Custom Name" bind:value={value['display_name']} />
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Id </span>
				<span class="text-secondary font-normal text-xs"
					>Client credential from the client ID section of the okta service configuration</span
				>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Secret </span>
				<span class="text-secondary font-normal text-xs"
					>from the CLIENT SECRETS section of the okta service configuration</span
				>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
			<CollapseLink text="Instructions">
				<div class="text-xs text-primary border rounded-md p-4 space-y-3">
					<div>
						<strong>1. Create App Integration</strong>
						<div class="ml-4 mt-1">
							From your Admin page, setup windmill using the service flow and create a new app
							integration:
							<ul class="list-disc ml-4 mt-1 space-y-1">
								<li>For "sign-in method" select <strong>OIDC - Open ID Connect</strong></li>
								<li>For "application type" select <strong>Web Application</strong></li>
							</ul>
						</div>
					</div>

					<div>
						<strong>2. Grant Type Configuration</strong>
						<div class="ml-4 mt-1">
							Select all of the following options for Grant type of "Client acting on behalf of a
							user":
							<ul class="list-disc ml-4 mt-1 space-y-1">
								<li>Authorization Code</li>
								<li>Refresh Token</li>
								<li>Implicit (hybrid)</li>
								<li>Allow ID Token with implicit grant type</li>
								<li>Allow Access Token with implicit grant type</li>
							</ul>
							For Refresh Token, select <strong>"Rotate token after every use"</strong>
						</div>
					</div>

					<div>
						<strong>3. Login Configuration</strong>
						<div class="ml-4 mt-1">
							Under "LOGIN", set the following:
							<ul class="list-disc ml-4 mt-1 space-y-1">
								<li
									><strong>Sign-in redirect URIs:</strong>
									<code class="bg-surface px-1 rounded text-xs"
										>BASE_URL/user/login_callback/okta</code
									></li
								>
								<li
									><strong>Sign-out redirect URIs:</strong>
									<code class="bg-surface px-1 rounded text-xs">BASE_URL/auth/logout</code></li
								>
								<li><strong>Login initiated by:</strong> App Only</li>
								<li
									><strong>Initiate login URI:</strong>
									<code class="bg-surface px-1 rounded text-xs">BASE_URL/user/login</code></li
								>
							</ul>
						</div>
					</div>
				</div>
			</CollapseLink>
		</SettingCard>
	{/if}
</div>
