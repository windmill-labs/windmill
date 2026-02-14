<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	export let value: any

	const AUTH_URL_SUFFIX = '/ui/oauth2'

	$: enabled = value != undefined

	// If `baseUrl` is not already set in the form, try to parse it from the `auth_url` value
	//
	// The binding dance here allows us to avoid rendering the string 'undefined' in the input, and
	// also allow lazy/async binding of the `value` prop.
	$: derivedBaseUrl = value?.connect_config?.auth_url?.replace(AUTH_URL_SUFFIX, '')
	let proxyUrlValue = undefined
	$: baseUrl = proxyUrlValue ?? derivedBaseUrl ?? ''

	$: changeValues({ baseUrl, id: value?.id ?? '' })

	function changeValues({ baseUrl, id }) {
		if (value) {
			value = {
				...value,
				connect_config: {
					auth_url: `${baseUrl}${AUTH_URL_SUFFIX}`,
					token_url: `${baseUrl}/oauth2/token`,
					scopes: ['openid', 'profile', 'email']
				},
				login_config: {
					auth_url: `${baseUrl}${AUTH_URL_SUFFIX}`,
					token_url: `${baseUrl}/oauth2/token`,
					userinfo_url: `${baseUrl}/oauth2/openid/${id}/userinfo`,
					scopes: ['openid', 'profile', 'email']
				}
			}

			proxyUrlValue = baseUrl
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name={'kanidm'} after={true} /></div><Toggle
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
		<SettingCard class="flex flex-col gap-6">
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Kanidm Url</span>
				<span class="text-secondary font-normal text-xs">{'KANIDM_URL/ui/oauth2'}</span>
				<TextInput inputProps={{ type: 'text', placeholder: 'Base URL' }} bind:value={baseUrl} />
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Custom Name' }}
					bind:value={value['display_name']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Id</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Id' }}
					bind:value={value['id']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Secret </span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Secret' }}
					bind:value={value['secret']}
				/>
			</label>
		</SettingCard>
	{/if}
</div>
