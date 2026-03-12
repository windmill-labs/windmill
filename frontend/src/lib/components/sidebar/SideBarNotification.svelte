<script lang="ts">
	import Notification from '$lib/components/common/alert/Notification.svelte'

	interface Props {
		notificationCount?: number;
		small?: boolean;
	}

	let { notificationCount = 0, small = false }: Props = $props();
</script>

{#if !small}
	<Notification {notificationCount} notificationLimit={9} />
{:else}
	<div class="bg-red-500 rounded-md w-3 h-3 flex items-center justify-center"></div>
{/if}
