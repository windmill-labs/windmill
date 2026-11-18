<script lang="ts">
	import Toggle from './Toggle.svelte'

	// Unset means off, so an instance that upgrades keeps how its logins worked.
	// The getter/setter binding writes only on click: opening the settings must not
	// turn an unset value into `false` and mark the form as changed. A form that
	// creates a new SSO provider entry sets `pkce: true` itself.
	let { pkce = $bindable() }: { pkce?: boolean } = $props()
</script>

<Toggle
	size="xs"
	options={{
		right: 'Send PKCE',
		rightTooltip:
			'Sends a code challenge (S256) with the login. Most identity providers accept it and some require it. Turn it off only if your provider rejects logins that carry it.'
	}}
	bind:checked={() => pkce ?? false, (v) => (pkce = v)}
/>
