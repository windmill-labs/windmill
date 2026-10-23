<script lang="ts">
	import NativeOAuthCallback from '$lib/components/triggers/native/NativeOAuthCallback.svelte'
</script>

<!-- Where connections made with the instance GitHub OAuth app landed before they used the resource
     connect callback: authorizations already in flight still finish here. -->
<NativeOAuthCallback />
