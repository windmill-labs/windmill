<script lang="ts">
	import Notification from '$lib/components/common/alert/Notification.svelte'

	interface Props {
		notificationCount?: number
		small?: boolean
	}

	let { notificationCount = 0, small = false }: Props = $props()
</script>

{#if !small}
	<Notification {notificationCount} notificationLimit={9} />
{:else}
	<div class="bg-red-500 rounded-full w-2 h-2 flex items-center justify-center"></div>
{/if}
