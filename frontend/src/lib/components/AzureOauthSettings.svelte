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

<label class="block pb-2" for="tenant-id">
	<span class="text-primary font-semibold text-sm flex gap-2 items-center">
		<a
			href="https://learn.microsoft.com/en-us/azure/active-directory/develop/quickstart-register-app"
			target="_blank"
		>
			Azure Tenant Id
		</a>
		<ExternalLink size={12} />
	</span>
	<input
		id="tenant-id"
		type="text"
		placeholder="Tenant Id"
		required={true}
		bind:value={connect_config.extra_params.tenant_id}
	/>
</label>
<label class="block pb-2" for="scopes-label">
	<span class="text-primary font-semibold text-sm">Scopes</span>
	<div id="scopes-label">
		<OauthScopes bind:scopes={connect_config.scopes} />
	</div>
</label>
