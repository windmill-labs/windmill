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
					auth_url: `${org}/application/o/authorize/`,
					token_url: `${org}/application/o/token/`,
					scopes: ['openid', 'offline_access']
				},
				login_config: {
					auth_url: `${org}/application/o/authorize/`,
					token_url: `${org}/application/o/token/`,
					userinfo_url: `${org}/application/o/userinfo/`,
					scopes: ['openid', 'offline_access', 'email']
				}
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<label class="text-sm font-medium text-primary flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name={'authentik'} after={true} /></div><Toggle
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
					>Authentik Url ({'AUTHENTIK_HOST/application/o/authorize/'})</span
				>
				<input type="text" placeholder="yourorg" bind:value={org} />
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
