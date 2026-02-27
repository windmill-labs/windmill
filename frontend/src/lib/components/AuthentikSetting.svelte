<script lang="ts">

	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface Props {
		value: any;
	}

	let { value = $bindable() }: Props = $props();




	function changeOrg(org) {
		if (value && org) {
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
	let enabled = $derived(value != undefined)
	// Initialize org from existing auth_url
	let org = $derived(value?.connect_config?.auth_url?.replace('/application/o/authorize/', '') ?? '')
	$effect(() => {
		changeOrg(org)
	});
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center"
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
		<SettingCard class="flex flex-col gap-6">
			<label>
				<span class="text-emphasis font-semibold text-xs">Authentik Url</span>
				<span class="text-secondary font-normal text-xs"
					>({'AUTHENTIK_HOST/application/o/authorize/'})</span
				>
				<input type="text" placeholder="Authentik base url" bind:value={org} required />
			</label>
			<label>
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<input type="text" placeholder="Custom Name" bind:value={value['display_name']} />
			</label>
			<label>
				<span class="text-emphasis font-semibold text-xs">Client Id</span>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label>
				<span class="text-emphasis font-semibold text-xs">Client Secret </span>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
		</SettingCard>
	{/if}
</div>
