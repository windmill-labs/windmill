<script lang="ts">
	import { untrack } from 'svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	let { value = $bindable() }: { value: any } = $props()

	let enabled = $derived(value != undefined)

	// Initialize org from existing config or empty string
	let org = $state(value?.connect_config?.auth_url?.replace('/authorize', '') ?? '')

	// Update configs when org changes - use untrack to avoid infinite loop
	$effect(() => {
		const currentOrg = org
		untrack(() => {
			if (value && currentOrg) {
				value = {
					...value,
					connect_config: {
						auth_url: `${currentOrg}/authorize`,
						token_url: `${currentOrg}/api/oidc/token`,
						scopes: ['openid', 'profile', 'email']
					},
					login_config: {
						auth_url: `${currentOrg}/authorize`,
						token_url: `${currentOrg}/api/oidc/token`,
						userinfo_url: `${currentOrg}/api/oidc/userinfo`,
						scopes: ['openid', 'profile', 'email']
					}
				}
			}
		})
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
		<div class="w-[120px]"><IconedResourceType name="pocketid" after={true} /></div>
		<Toggle checked={enabled} on:change={handleToggle} />
	</label>
	{#if enabled}
		<SettingCard class="flex flex-col gap-6">
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Pocket ID Url</span>
				<span class="text-secondary font-normal text-xs">POCKET_ID_URL/authorize</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'https://id.example.com' }}
					bind:value={org}
				/>
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
		</SettingCard>
	{/if}
</div>
