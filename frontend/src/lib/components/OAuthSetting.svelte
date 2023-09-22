<script lang="ts">
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import Toggle from './Toggle.svelte'

	export let name: string
	export let value: any
	export let login = true

	$: enabled = value != undefined

	
	let allowed_domains = value?.['allowed_domains'] ?? ''
</script>

<div class="flex flex-col gap-1">
	<label class="text-sm flex gap-4 items-center font-medium text-gray-700"
		><div class="w-[120px]"><IconedResourceType {name} after={true} /></div><Toggle
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
		<div class="p-2 rounded border">
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Client Id</span>
				<input type="text" placeholder="Client Id" bind:value={value['id']} />
			</label>
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">Client Secret</span>
				<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
			</label>
			{#if login}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Allowed domain</span>
					<input
						type="text"
						placeholder="windmill.dev"
						bind:value={allowed_domains}
						on:keyup={() => {
							if (allowed_domains == '') {
								value['allowed_domains'] = undefined
							} else {
								value['allowed_domains'] = [allowed_domains]
							}
						}}
					/>
				</label>
			{/if}
			{#if name == 'google'}
				<CollapseLink text="Instructions">
					<div class="text-sm text-secondary border p-2">
						Create a new OAuth 2.0 Client <a
							href="https://console.cloud.google.com/apis/credentials"
							target="_blank">in google console</a
						>
						and set the redirect URI to <code>BASE_URL/user/login_callback/google</code>
						where BASE_URL is what you configured as core BASE_URL
					</div>
				</CollapseLink>
			{:else if name == 'slack'}
				<a class="text-xs" href="https://www.windmill.dev/docs/misc/setup_oauth#slack"
					>Read more about Slack OAuth on the docs</a
				>
			{:else if name == 'microsoft'}
				<CollapseLink text="Instructions">
					<div class="text-sm text-secondary border p-2">
						Create a new OAuth 2.0 Client <a
							href="https://portal.azure.com/#blade/Microsoft_AAD_RegisteredApps/ApplicationsListBlade"
							target="_blank">in microsoft portal</a
						>
						and in the "Authentication" tab, set the redirect URI to
						<code>BASE_URL/user/login_callback/microsoft</code>, the logout channel to
						<code>BASE_URL/auth/logout</code>where BASE_URL is what you configured as core BASE_URL.
						Also set "Accounts in any organizational directory (Any Microsoft Entra ID tenant -
						Multitenant) and personal Microsoft accounts (e.g. Skype, Xbox)", you can restrict the
						emails directly in windmill using the "allowed_domains" setting.
					</div>
				</CollapseLink>
			{/if}
		</div>
	{/if}
</div>

