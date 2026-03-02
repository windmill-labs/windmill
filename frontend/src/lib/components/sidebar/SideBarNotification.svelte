<script lang="ts">
	import Notification from '$lib/components/common/alert/Notification.svelte'

	export let notificationCount = 0
	export let small: boolean = false
</script>

{#if !small}
	<Notification {notificationCount} notificationLimit={9} />
{:else}
	<div class="bg-red-500 rounded-md w-3 h-3 flex items-center justify-center"></div>
{/if}
