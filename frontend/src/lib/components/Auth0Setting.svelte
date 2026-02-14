<script lang="ts">
	import { untrack } from 'svelte'
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface Props {
		value: any
	}

	let { value = $bindable() }: Props = $props()

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
	let enabled = $derived(value != undefined)
	let lastValues = { domain: undefined, custom: undefined }
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
								Org ({'https://<your org>.auth0.com'}){/if}</span
						>
					</div>
				</div>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Custom Name' }}
					bind:value={value['display_name']}
					class="max-w-lg"
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client ID </span>
				<span class="text-secondary font-normal text-xs"
					>Client ID credential of the auth0 service configuration</span
				>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Id' }}
					bind:value={value['id']}
					class="max-w-lg"
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs"
					>Client Secret <Tooltip>Client Secret of the auth0 service configuration</Tooltip></span
				>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Secret' }}
					bind:value={value['secret']}
					class="max-w-lg"
				/>
			</label>
			<CollapseLink text="Instructions">
				<div class="text-xs text-primary border rounded-md p-4 space-y-3">
					<div>
						<strong>1. Create Application</strong>
						<div class="ml-4 mt-1">
							From your auth0 Admin page, setup a Windmill application:
							<ul class="list-disc ml-4 mt-1 space-y-1">
								<li>Create a new application</li>
								<li>For "application type" select <strong>Regular Web Application</strong></li>
								<li
									>Copy down the "Client ID" and "Client Secret" and paste them into the fields
									above</li
								>
							</ul>
						</div>
					</div>

					<div>
						<strong>2. Application URIs Configuration</strong>
						<div class="ml-4 mt-1">
							Under "Application URIs", set the following:
							<ul class="list-disc ml-4 mt-1 space-y-1">
								<li
									><strong>Application Login URI:</strong>
									<code class="bg-surface px-1 rounded text-xs">BASE_URL/user/login</code></li
								>
								<li
									><strong>Allowed Callback URLs:</strong>
									<code class="bg-surface px-1 rounded text-xs"
										>BASE_URL/user/login_callback/auth0</code
									></li
								>
								<li
									><strong>Allowed Logout URLs:</strong>
									<code class="bg-surface px-1 rounded text-xs">BASE_URL/auth/logout</code></li
								>
								<li
									><strong>Allowed Web Origins:</strong>
									<code class="bg-surface px-1 rounded text-xs">BASE_URL</code></li
								>
								<li
									><strong>Allowed Origins (CORS):</strong>
									<code class="bg-surface px-1 rounded text-xs">BASE_URL</code></li
								>
							</ul>
						</div>
					</div>
				</div>
			</CollapseLink>
		</SettingCard>
	{/if}
</div>
