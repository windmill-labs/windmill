<script lang="ts">
	interface Props {
		/** undefined until the instance's login options are known. */
		hasThirdParty: boolean | undefined
	}

	let { hasThirdParty }: Props = $props()
</script>

<!-- Held blank rather than defaulted while the options load: a third-party login also creates
	the account, so guessing either way flashes copy that is wrong for half the instances. -->
<div class="min-h-14">
	{#if hasThirdParty !== undefined}
		<h2 class="text-center text-2xl font-semibold tracking-tight text-emphasis">
			{hasThirdParty ? 'Log in or sign up' : 'Log in'}
		</h2>
		<p class="mt-2 text-center text-xs text-secondary">
			{hasThirdParty
				? 'Log in or sign up with any of the methods below'
				: 'Log in with your email and password'}
		</p>
	{/if}
</div>
