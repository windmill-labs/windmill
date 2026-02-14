<script lang="ts">
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface Props {
		value: any
		baseUrl?: string
	}

	let { value = $bindable(), baseUrl }: Props = $props()

	function changeDomain(domain) {
		if (value && domain) {
			// Remove http:// or https:// if user included it
			const cleanDomain = domain.replace(/^https?:\/\//, '')
			let baseUrl = `https://${cleanDomain}`
			value = {
				...value,
				login_config: {
					auth_url: `${baseUrl}/apps/oauth2/authorize`,
					token_url: `${baseUrl}/apps/oauth2/api/v1/token`,
					userinfo_url: `${baseUrl}/ocs/v2.php/cloud/user?format=json`,
					scopes: []
				},
				connect_config: {
					auth_url: `${baseUrl}/apps/oauth2/authorize`,
					token_url: `${baseUrl}/apps/oauth2/api/v1/token`,
					scopes: []
				}
			}
		}
	}
	let enabled = $derived(value != undefined)
	let lastDomain = $state(value?.['domain'])

	$effect(() => {
		const currentDomain = value?.['domain']
		if (currentDomain && currentDomain !== lastDomain) {
			lastDomain = currentDomain
			changeDomain(currentDomain)
		}
	})
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name="nextcloud" after={true} /></div><Toggle
			checked={enabled}
			on:change={(e) => {
				if (e.detail) {
					value = { id: '', secret: '', domain: '' }
				} else {
					value = undefined
				}
			}}
		/></label
	>
	{#if enabled}
		<SettingCard class="flex flex-col gap-6">
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Nextcloud Instance Domain</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'example.nextcloud.com' }}
					bind:value={value['domain']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Custom Name' }}
					bind:value={value['display_name']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Id </span>
				<span class="text-secondary font-normal text-xs"
					>Client ID from your Nextcloud OAuth2 app configuration</span
				>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Id' }}
					bind:value={value['id']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Secret </span>
				<span class="text-secondary font-normal text-xs"
					>Client Secret from your Nextcloud OAuth2 app configuration</span
				>
				<TextInput
					inputProps={{ type: 'password', placeholder: 'Client Secret' }}
					bind:value={value['secret']}
				/>
			</label>
			<CollapseLink text="Instructions">
				<div class="text-xs text-primary border rounded-md p-4">
					1. Go to your Nextcloud instance as an administrator<br />
					2. Navigate to <strong>Administration settings → Security → OAuth 2.0 clients</strong><br
					/>
					3. Click "Add client" to create a new OAuth2 application<br />
					4. Set the redirect URI to your Windmill instance's
					<code>{baseUrl || 'BASE_URL'}/user/login_callback/nextcloud</code><br />
					5. Copy the Client ID and Client Secret to the fields above<br />
				</div>
			</CollapseLink>
		</SettingCard>
	{/if}
</div>
