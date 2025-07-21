<script lang="ts">
	import OauthExtraParams from './OauthExtraParams.svelte'
	import OauthScopes from './OauthScopes.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'

	export let connect_config = {
		scopes: [],
		auth_url: '',
		token_url: '',
		req_body_auth: false,
		extra_params: {},
		extra_params_callback: {}
	}

	$: if (!connect_config) {
		connect_config = {
			scopes: [],
			auth_url: '',
			token_url: '',
			req_body_auth: false,
			extra_params: {},
			extra_params_callback: {}
		}
	}
</script>

<label class="block pb-2">
	<span class="text-primary font-semibold text-sm">Auth URL</span>
	<input
		type="text"
		placeholder="https://github.com/login/oauth/authorize"
		bind:value={connect_config.auth_url}
	/>
</label>
<label class="block pb-2">
	<span class="text-primary font-semibold text-sm">Token URL</span>
	<input
		type="text"
		placeholder="https://github.com/login/oauth/access_token"
		bind:value={connect_config.token_url}
	/>
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-2">
	<span class="text-primary font-semibold text-sm">Scopes</span>
	<OauthScopes bind:scopes={connect_config.scopes} />
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-2">
	<span class="text-primary font-semibold text-sm"
		>Extra Query Args for Authorize Request&nbsp;<Tooltip
			>Not needed in most cases. Examples of uses: google apis require the 2 extra args
			"access_type=offline&prompt=consent"</Tooltip
		></span
	>
	<OauthExtraParams bind:extra_params={connect_config.extra_params} />
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-2">
	<span class="text-primary font-semibold text-sm"
		>Extra Query Args for Token request <Tooltip>Not needed in most cases</Tooltip></span
	>
	<OauthExtraParams bind:extra_params={connect_config.extra_params_callback} />
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-2">
	<span class="text-primary font-semibold text-sm"
		>Payload <Tooltip
			>Auth (client id/client secret) is passed as basic auth most commonly but can be passed in the
			body x-www-form-urlencoded. Some LinkedIn is an example of OAuth using x-www-form-urlencoded
		</Tooltip></span
	>
	<div>
		<Toggle
			options={{ left: 'in query args', right: 'in body x-www-form-urlencoded' }}
			bind:checked={connect_config.req_body_auth}
		/></div
	>
</label>
