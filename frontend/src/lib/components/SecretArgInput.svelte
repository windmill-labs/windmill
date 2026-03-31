<script lang="ts">
	import type { Snippet } from 'svelte'
	import { userStore } from '$lib/stores'

	interface Props {
		value?: any
		children?: Snippet
	}

	let { value = $bindable(), children }: Props = $props()

	let userPrefix = $derived(
		'u/' + ($userStore?.username ?? $userStore?.email)?.split('@')[0] + '/secret_arg/'
	)

	let isAlreadySecret = $derived(typeof value === 'string' && value.startsWith('$jsonvar:'))
</script>

{@render children?.()}
{#if isAlreadySecret}
	<div class="text-2xs text-tertiary">
		Sensitive — stored as secret: <code class="text-2xs">{value.slice('$jsonvar:'.length)}</code>
	</div>
{:else}
	<div class="text-2xs text-tertiary italic"> Sensitive — will be stored as secret on submit </div>
{/if}
