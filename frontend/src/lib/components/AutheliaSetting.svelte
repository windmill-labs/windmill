<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	export let value: any

	$: enabled = value != undefined

	let org = ''

	$: changeOrg(org)

	function changeOrg(org) {
		if (value) {
			value = {
				...value,
				connect_config: {
					auth_url: `${org}/api/oidc/authorization`,
					token_url: `${org}/api/oidc/token`,
					scopes: ['openid', 'profile', 'email', 'groups']
				},
				login_config: {
					auth_url: `${org}/api/oidc/authorization`,
					token_url: `${org}/api/oidc/token`,
					userinfo_url: `${org}/api/oidc/userinfo`,
					scopes: ['openid', 'profile', 'email', 'groups']
				}
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
		><div class="w-[120px]"><IconedResourceType name={'authelia'} after={true} /></div><Toggle
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
				<span class="text-emphasis font-semibold text-xs">Authelia Url</span>
				<span class="text-secondary font-normal text-xs"
					>{'AUTHELIA_URL/api/oidc/authorization'}</span
				>
				<TextInput inputProps={{ type: 'text', placeholder: 'yourorg' }} bind:value={org} />
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
