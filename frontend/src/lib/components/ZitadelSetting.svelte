<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'

	export let value: any

	$: enabled = value != undefined

	let org = ''

	$: changeOrg(org)

	function changeOrg(org) {
		if (value) {
			value = {
				...value,
				connect_config: {
					auth_url: `${org}/oauth/v2/authorize`,
					token_url: `${org}/oauth/v2/token`,
					scopes: ['openid', 'profile', 'email']
				},
				login_config: {
					auth_url: `${org}/oauth/v2/authorize`,
					token_url: `${org}/oauth/v2/token`,
					userinfo_url: `${org}/oidc/v1/userinfo`,
					scopes: ['openid', 'profile', 'email']
				}
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name={'zitadel'} after={true} /></div><Toggle
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
				<span class="text-primary font-semibold text-sm"
					>Zitadel Url ({'ZITADEL_URL/oauth/v2/authorize'})</span
				>
				<input type="text" placeholder="yourorg" bind:value={org} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Custom Name</span>
				<input type="text" placeholder="Custom Name" bind:value={value['display_name']} />
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
