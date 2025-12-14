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
					auth_url: `${org}/protocol/openid-connect/auth`,
					token_url: `${org}/protocol/openid-connect/token`,
					scopes: ['openid', 'offline_access']
				},
				login_config: {
					auth_url: `${org}/protocol/openid-connect/auth`,
					token_url: `${org}/protocol/openid-connect/token`,
					userinfo_url: `${org}/protocol/openid-connect/userinfo`,
					scopes: ['openid', 'offline_access']
				}
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name={'keycloak'} after={true} /></div><Toggle
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
					>Realm Url ({'REALM_URL/protocol/openid-connect/auth'})</span
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
