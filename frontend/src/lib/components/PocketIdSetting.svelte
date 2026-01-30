<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'

	let { value = $bindable() }: { value: any } = $props()

	let enabled = $derived(value != undefined)

	// Initialize org from existing config or empty string
	let org = $state(value?.connect_config?.auth_url?.replace('/authorize', '') ?? '')

	// Update configs when org changes
	$effect(() => {
		if (value && org) {
			value = {
				...value,
				connect_config: {
					auth_url: `${org}/authorize`,
					token_url: `${org}/api/oidc/token`,
					scopes: ['openid', 'profile', 'email']
				},
				login_config: {
					auth_url: `${org}/authorize`,
					token_url: `${org}/api/oidc/token`,
					userinfo_url: `${org}/api/oidc/userinfo`,
					scopes: ['openid', 'profile', 'email']
				}
			}
		}
	})

	function handleToggle(e: CustomEvent<boolean>) {
		if (e.detail) {
			value = { id: '', secret: '' }
			org = ''
		} else {
			value = undefined
			org = ''
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center">
		<div class="w-[120px]"><IconedResourceType name={'pocketid'} after={true} /></div>
		<Toggle checked={enabled} onchange={handleToggle} />
	</label>
	{#if enabled}
		<div class="border rounded p-4 flex flex-col gap-6">
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Pocket ID Url</span>
				<span class="text-secondary font-normal text-xs">{'POCKET_ID_URL/authorize'}</span>
				<TextInput inputProps={{ type: 'text', placeholder: 'https://id.example.com' }} bind:value={org} />
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Pocket ID' }}
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
				<span class="text-emphasis font-semibold text-xs">Client Secret</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Client Secret' }}
					bind:value={value['secret']}
				/>
			</label>
		</div>
	{/if}
</div>
