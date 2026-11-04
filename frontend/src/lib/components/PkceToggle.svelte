<script lang="ts">
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'

	// Unset means off, so an instance that upgrades keeps how its logins worked.
	// The getter/setter binding writes only on click: opening the settings must not
	// turn an unset value into `false` and mark the form as changed. A form that
	// creates a new SSO provider entry sets `pkce: true` itself.
	let { pkce = $bindable() }: { pkce?: boolean } = $props()
</script>

<label class="flex flex-col gap-1">
	<span class="text-emphasis font-semibold text-xs"
		>PKCE <Tooltip
			>Sends a code challenge (S256) with the login. Most identity providers accept it and some
			require it. Turn it off only if your provider rejects logins that carry it.</Tooltip
		></span
	>
	<div>
		<Toggle
			options={{ right: 'Send PKCE' }}
			bind:checked={() => pkce ?? false, (v) => (pkce = v)}
		/>
	</div>
</label>
