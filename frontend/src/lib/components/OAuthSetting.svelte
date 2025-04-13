<script lang="ts">
	import { X } from 'lucide-svelte'
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import { onMount } from 'svelte'
	import { enterpriseLicense } from '$lib/stores'

	export let name: string
	export let value: any
	export let login = true
	export let eeOnly = false

	$: enabled = value != undefined && !(eeOnly && !$enterpriseLicense)

	let tenant: string = ''
	$: (name == 'microsoft' || name == 'teams') && changeTenantId(tenant)

	onMount(() => {
		try {
			if (
				name == 'microsoft' &&
				value &&
				value['login_config'] &&
				typeof value['login_config']['auth_url'] == 'string'
			) {
				tenant = value['login_config']['auth_url'].split('/')[3]
			}
			if (name === 'teams' && value?.tenant) {
				tenant = value.tenant
			}
		} catch (e) {
			console.error('Could not set tenantId', e)
		}
	})

	function changeTenantId(tenant: string) {
		if (value && tenant) {
			if (tenant != '') {
				if (name === 'teams') {
					value = {
						...value,
						tenant
					}
				} else {
					value = {
						...value,
						login_config: {
							auth_url: `https://login.microsoftonline.com/${tenant}/oauth2/v2.0/authorize`,
							token_url: `https://login.microsoftonline.com/${tenant}/oauth2/v2.0/token`,
							userinfo_url: `https://graph.microsoft.com/oidc/userinfo`,
							scopes: ['openid', 'profile', 'email']
						}
					}
				}
			} else {
				if (value['login_config']) {
					delete value['login_config']
				}
			}
		}
	}
</script>

<div class="flex flex-col">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label
		class="text-sm flex gap-4 items-center font-medium text-primary {enabled ? 'rounded py-2' : ''}"
		><div class="w-[120px]"><IconedResourceType {name} after={true} /></div><Toggle
			checked={enabled}
			disabled={eeOnly && !$enterpriseLicense}
			on:change={(e) => {
				if (e.detail) {
					if (name === 'teams' || name === 'microsoft') {
						value = { id: '', secret: '', tenant: '' }
					} else {
						value = { id: '', secret: '' }
					}
				} else {
					value = undefined
				}
			}}
		/>
		{#if eeOnly && !$enterpriseLicense}
			<div class="text-xs"> (EE only) </div>
		{/if}
	</label>
	{#if enabled}
		<div class="p-2 rounded border mb-4">
			{#if name != 'slack' && name != 'teams'}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Custom Name</span>
					<input type="text" placeholder="Custom Name" bind:value={value['display_name']} />
				</label>
			{/if}
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Client Id</span>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Client Secret</span>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
			{#if name == 'microsoft' || name == 'teams'}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Tenant Id</span>
					<input type="text" placeholder="Tenant Id" bind:value={tenant} />
				</label>
			{:else if login}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Allowed domains</span>
					<div class="flex flex-col gap-1">
						{#each value?.['allowed_domains'] ?? [] as domain, idx}
							<div class="flex gap-2">
								<input
									class="max-w-96 w-full"
									type="text"
									bind:value={value['allowed_domains'][idx]}
									on:keyup={(e) => {
										if (domain == '') {
											value['allowed_domains'] = value['allowed_domains']?.filter(
												(d) => d != domain
											)
										}
									}}
								/>
								<button
									class="text-primary text-xs rounded hover:bg-surface-hover"
									on:click={() => {
										value['allowed_domains'] = value['allowed_domains']?.filter((d) => d != domain)
										if (value['allowed_domains'].length == 0) {
											value['allowed_domains'] = undefined
										}
									}}
								>
									<X size={14} />
								</button>
							</div>
						{/each}
						<div class="flex gap-2">
							<button
								class="text-primary text-sm border rounded p-1"
								on:click={() => {
									value['allowed_domains'] = [...(value['allowed_domains'] ?? []), 'mydomain.com']
								}}>+ Add domain</button
							>
						</div>
					</div>
				</label>
			{/if}
			{#if name == 'google'}
				<CollapseLink text="Instructions">
					<div class="text-sm text-secondary border p-2">
						Create a new OAuth 2.0 Client <a
							href="https://console.cloud.google.com/apis/credentials"
							target="_blank">in Google console</a
						>
						and set the redirect URI to <code>BASE_URL/user/login_callback/google</code>
						where BASE_URL is what you configured as core BASE_URL
					</div>
				</CollapseLink>
			{:else if name == 'slack'}
				<CollapseLink text="Instructions">
					<div class="text-sm text-secondary border p-2">
						Create a new App <a href="https://api.slack.com/apps?new_app=1" target="_blank"
							>in Slack API Console</a
						>. Pick "From an app manifest", then YAML and paste manifest template found on
						<a href="https://www.windmill.dev/docs/misc/setup_oauth#slack" target="_blank"
							>Windmill Docs</a
						> and then paste Client ID and Client Secret here.
					</div>
				</CollapseLink>
			{:else if name == 'microsoft'}
				<CollapseLink text="Instructions">
					<div class="text-sm text-secondary border p-2">
						Create a new OAuth 2.0 Client <a
							href="https://portal.azure.com/#blade/Microsoft_AAD_RegisteredApps/ApplicationsListBlade"
							target="_blank">in Microsoft portal</a
						>
						"Add" {'->'} "App Registration" -> "Accounts in this organizational directory only (Default
						Directory only - Single tenant)", and in the "Authentication" tab, set the redirect URI to
						Web and
						<code>BASE_URL/user/login_callback/microsoft</code>. Then copy the "Directory (tenant
						ID)" in the tenant ID field. Then copy the Client ID from "Application (client) ID" and
						create a secret in "Client credentials". Last, include "Sign in" and "read user profile"
						under "Delegated Permissions".
					</div>
				</CollapseLink>
			{:else if name == 'teams'}
				<CollapseLink text="Instructions">
					<div class="text-sm text-secondary border p-2">
						Follow this guide on <a
							href="https://www.windmill.dev/docs/misc/setup_oauth#microsoft-teams"
							target="_blank">Windmill Docs</a
						> to create a new Microsoft Teams App. Then paste Client ID, Tenant ID, and Client Secret
						here.
					</div>
				</CollapseLink>
			{/if}
		</div>
	{/if}
</div>
