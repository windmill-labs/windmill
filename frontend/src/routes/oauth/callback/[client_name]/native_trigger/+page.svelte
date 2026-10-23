<script lang="ts">
	import { page } from '$app/state'
	import NativeOAuthCallback from '$lib/components/triggers/native/NativeOAuthCallback.svelte'
</script>

<!-- Where native trigger connections made with the instance GitHub OAuth app land: that app only
     redirects below its registered callback, /oauth/callback/github. -->
<NativeOAuthCallback redirectUri="{page.url.origin}/oauth/callback/github/native_trigger" />
