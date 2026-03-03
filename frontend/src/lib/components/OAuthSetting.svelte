<script lang="ts">
	import { ExternalLink, Plus, X } from 'lucide-svelte'
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import { onMount, untrack } from 'svelte'
	import { enterpriseLicense } from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface Props {
		name: string
		value: any
		login?: boolean
		eeOnly?: boolean
	}

	let { name, value = $bindable(), login = true, eeOnly = false }: Props = $props()

	let tenant: string = $state('')

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

	let enabled = $derived(value != undefined && !(eeOnly && !$enterpriseLicense))

	$effect(() => {
		tenant
		if (name == 'microsoft' || name == 'teams') {
			untrack(() => changeTenantId(tenant))
		}
	})
</script>

<div class="flex flex-col gap-2">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label
		class="text-xs flex gap-4 items-center font-semibold text-emphasis {enabled ? 'rounded' : ''}"
		><div class="w-[120px]"><IconedResourceType {name} after={true} /></div>
		<Toggle
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
		<SettingCard class="mb-4 flex flex-col gap-6">
			{#if name != 'slack' && name != 'teams'}
				<label class="flex flex-col gap-1">
					<span class="text-emphasis font-semibold text-xs">Custom Name</span>
					<TextInput
						inputProps={{ type: 'text', placeholder: 'Custom Name' }}
						bind:value={value['display_name']}
					/>
				</label>
			{/if}
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Id</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Id' }}
					bind:value={value['id']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Secret</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Secret' }}
					bind:value={value['secret']}
				/>
			</label>
			{#if name == 'microsoft' || name == 'teams'}
				<label class="flex flex-col gap-1">
					<span class="text-emphasis font-semibold text-xs">Tenant Id</span>
					<TextInput inputProps={{ type: 'text', placeholder: 'Tenant Id' }} bind:value={tenant} />
				</label>
			{:else if login}
				<label class="flex flex-col gap-1">
					<span class="text-emphasis font-semibold text-xs">Allowed domains</span>
					<div class="flex flex-col gap-1">
						{#each value?.['allowed_domains'] ?? [] as domain, idx}
							<div class="flex gap-2">
								<input
									class="max-w-96 w-full"
									type="text"
									bind:value={value['allowed_domains'][idx]}
									onkeyup={(e) => {
										if (domain == '') {
											value['allowed_domains'] = value['allowed_domains']?.filter(
												(d) => d != domain
											)
										}
									}}
								/>
								<button
									class="text-primary text-xs rounded hover:bg-surface-hover"
									onclick={() => {
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
							<Button
								variant="default"
								unifiedSize="md"
								startIcon={{ icon: Plus }}
								onclick={() => {
									value['allowed_domains'] = [...(value['allowed_domains'] ?? []), 'mydomain.com']
								}}
								>Add domain
							</Button>
						</div>
					</div>
				</label>
			{/if}
			{#if name == 'google'}
				<CollapseLink text="Instructions">
					<div class="text-xs text-primary rounded-md">
						Create a new OAuth 2.0 Client <a
							href="https://console.cloud.google.com/apis/credentials"
							target="_blank">in Google console</a
						>
						and set the redirect URI to <code>BASE_URL/user/login_callback/google</code>
						where BASE_URL is what you configured as core BASE_URL
					</div>
				</CollapseLink>
			{:else if name == 'slack'}
				<CollapseLink text="Set up slack">
					<div class="text-xs text-primary rounded-md">
						To use Slack OAuth, create a new Slack app <a
							href="https://api.slack.com/apps?new_app=1"
							target="_blank"
							>in slack API console
							<ExternalLink size={12} class="inline-block" />
						</a>. Pick "From a manifest", then YAML and paste manifest template found on
						<a href="https://www.windmill.dev/docs/misc/setup_oauth#slack" target="_blank"
							>Windmill docs <ExternalLink size={12} class="inline-block" /></a
						> and then paste Client ID and Client Secret here.
					</div>
				</CollapseLink>
			{:else if name == 'microsoft'}
				<CollapseLink text="Instructions">
					<div class="text-xs text-primary border rounded-md p-4 space-y-3">
						<div>
							<strong>1. Create App Registration</strong>
							<div class="ml-4 mt-1">
								Create a new OAuth 2.0 Client <a
									href="https://portal.azure.com/#blade/Microsoft_AAD_RegisteredApps/ApplicationsListBlade"
									target="_blank"
									class="inline-flex items-center gap-1 whitespace-nowrap">in Microsoft portal</a
								>:
								<ul class="list-disc ml-4 mt-1 space-y-1">
									<li>Click <strong>"Add"</strong> → <strong>"App Registration"</strong></li>
									<li
										>Select <strong
											>"Accounts in this organizational directory only (Default Directory only -
											Single tenant)"</strong
										></li
									>
								</ul>
							</div>
						</div>

						<div>
							<strong>2. Authentication Configuration</strong>
							<div class="ml-4 mt-1">
								In the <strong>"Authentication"</strong> tab:
								<ul class="list-disc ml-4 mt-1 space-y-1">
									<li>Set the redirect URI to <strong>Web</strong></li>
									<li
										>Add redirect URI: <code class="bg-surface px-1 rounded text-xs"
											>BASE_URL/user/login_callback/microsoft</code
										></li
									>
								</ul>
							</div>
						</div>

						<div>
							<strong>3. Copy Credentials</strong>
							<div class="ml-4 mt-1">
								Copy the following values to Windmill:
								<ul class="list-disc ml-4 mt-1 space-y-1">
									<li>Copy <strong>"Directory (tenant ID)"</strong> to the tenant ID field</li>
									<li>Copy <strong>"Application (client) ID"</strong> to the Client ID field</li>
									<li
										>Create a secret in <strong>"Client credentials"</strong> and copy to Client Secret
										field</li
									>
								</ul>
							</div>
						</div>

						<div>
							<strong>4. API Permissions</strong>
							<div class="ml-4 mt-1">
								Under <strong>"Delegated Permissions"</strong>, include:
								<ul class="list-disc ml-4 mt-1 space-y-1">
									<li>Sign in</li>
									<li>Read user profile</li>
								</ul>
							</div>
						</div>
					</div>
				</CollapseLink>
			{:else if name == 'teams'}
				<CollapseLink text="Instructions">
					<div class="text-xs text-primary rounded-md">
						Follow this guide on <a
							href="https://www.windmill.dev/docs/misc/setup_oauth#microsoft-teams"
							target="_blank">Windmill Docs</a
						> to create a new Microsoft Teams App. Then paste Client ID, Tenant ID, and Client Secret
						here.
					</div>
				</CollapseLink>
			{/if}
		</SettingCard>
	{/if}
</div>
