<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import OauthScopes from './OauthScopes.svelte'

	export let connect_config: {
		scopes: string[]
		auth_url: string
		token_url: string
		req_body_auth: boolean
		extra_params: { tenant_id: string }
		extra_params_callback: Record<string, any>
	} = {
		scopes: ['offline_access'],
		auth_url: '',
		token_url: '',
		req_body_auth: true,
		extra_params: { tenant_id: '' },
		extra_params_callback: {}
	}

	$: if (!connect_config) {
		connect_config = {
			scopes: ['offline_access'],
			auth_url: '',
			token_url: '',
			req_body_auth: true,
			extra_params: { tenant_id: '' },
			extra_params_callback: {}
		}
	}

	$: if (connect_config.extra_params.tenant_id) {
		connect_config.auth_url = `https://login.microsoftonline.com/${connect_config.extra_params.tenant_id}/oauth2/v2.0/authorize`
		connect_config.token_url = `https://login.microsoftonline.com/${connect_config.extra_params.tenant_id}/oauth2/v2.0/token`
	}
</script>

<label class="flex flex-col gap-1" for="tenant-id">
	<span class="text-primary font-semibold text-xs flex gap-2 items-center"> Azure tenant id </span>
	<span class="text-secondary font-normal text-xs">
		Identifies the specific Microsoft Entra ID tenant for authentication. Controls who can sign into
		the application and determines the organizational boundary for OAuth requests.
		<a
			href="https://learn.microsoft.com/en-us/azure/active-directory/develop/quickstart-register-app"
			target="_blank"
			class="inline-flex items-center gap-1 whitespace-nowrap"
		>
			Learn more
			<ExternalLink size={12} />
		</a>
	</span>
	<input
		id="tenant-id"
		type="text"
		placeholder="Tenant Id"
		required={true}
		bind:value={connect_config.extra_params.tenant_id}
	/>
</label>
<label class="flex flex-col gap-1" for="scopes-label">
	<span class="text-primary font-semibold text-xs">Scopes</span>
	<div id="scopes-label">
		<OauthScopes bind:scopes={connect_config.scopes} />
	</div>
</label>
