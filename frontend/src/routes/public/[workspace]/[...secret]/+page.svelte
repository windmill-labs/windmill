<script lang="ts">
	import { goto, replaceState } from '$app/navigation'
	import { page } from '$app/stores'
	import AppEmbed from '$lib/components/AppEmbed.svelte'
</script>

<AppEmbed
	workspace={$page.params.workspace}
	secret={$page.params.secret}
	replaceStateFn={(path) => replaceState(path, $page.state)}
	gotoFn={(path, opt) => goto(path, opt)}
/>
