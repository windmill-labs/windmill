<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'

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
	<label class="text-sm font-medium text-primary flex gap-4 items-center"
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
		<div class="border rounded p-2">
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Kanidm Url ({'KANIDM_URL/ui/oauth2'})</span
				>
				<input type="text" placeholder="Base URL" bind:value={baseUrl} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Client Id</span>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Client Secret </span>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
		</div>
	{/if}
</div>
