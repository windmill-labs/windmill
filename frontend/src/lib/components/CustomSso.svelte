<script lang="ts">
	import OauthExtraParams from './OauthExtraParams.svelte'
	import OauthScopes from './OauthScopes.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'

	export let login_config = {
		scopes: [],
		auth_url: '',
		token_url: '',
		userinfo_url: '',
		req_body_auth: false,
		extra_params: {},
		extra_params_callback: {}
	}

	$: if (!login_config) {
		login_config = {
			scopes: [],
			auth_url: '',
			token_url: '',
			userinfo_url: '',
			req_body_auth: false,
			extra_params: {},
			extra_params_callback: {}
		}
	}
</script>

<label class="block pb-6">
	<span class="text-primary font-semibold text-xs">Auth URL</span>
	<input
		type="text"
		placeholder="https://github.com/login/oauth/authorize"
		bind:value={login_config.auth_url}
	/>
</label>
<label class="block pb-6">
	<span class="text-primary font-semibold text-xs">Token URL</span>
	<input
		type="text"
		placeholder="https://github.com/login/oauth/access_token"
		bind:value={login_config.token_url}
	/>
</label>
<label class="block pb-6">
	<span class="text-primary font-semibold text-xs">Userinfo URL</span>
	<input
		type="text"
		placeholder="https://github.com/login/oauth/userinfo"
		bind:value={login_config.userinfo_url}
	/>
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-6">
	<span class="text-primary font-semibold text-xs">Scopes</span>
	<OauthScopes bind:scopes={login_config.scopes} />
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-6">
	<span class="text-primary font-semibold text-xs"
		>Extra Query Args for Authorize Request&nbsp;<Tooltip
			>Not needed in most cases. Examples of uses: google apis require the 2 extra args
			"access_type=offline&prompt=consent"</Tooltip
		></span
	>
	<OauthExtraParams bind:extra_params={login_config.extra_params} />
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-6">
	<span class="text-primary font-semibold text-xs"
		>Extra Query Args for Token request <Tooltip>Not needed in most cases</Tooltip></span
	>
	<OauthExtraParams bind:extra_params={login_config.extra_params_callback} />
</label>
<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="block pb-6">
	<span class="text-primary font-semibold text-xs"
		>Payload <Tooltip
			>Auth is passed in query most commonly. LinkedIn is an example of OAuth using
			x-www-form-urlencoded
		</Tooltip></span
	>
	<div>
		<Toggle
			options={{ left: 'in query args', right: 'in body x-www-form-urlencoded' }}
			bind:checked={login_config.req_body_auth}
		/></div
	>
</label>
