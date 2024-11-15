<script lang="ts">
	export let notificationCount = 0
	export let small: boolean = false
</script>

{#if !small}
	<div
		class="bg-red-500 text-white text-[0.6rem] rounded-md w-5 h-5 flex items-center justify-center"
	>
		{notificationCount > 9 ? '9+' : notificationCount}
	</div>
{:else}
	<div class="bg-red-500 rounded-md w-3 h-3 flex items-center justify-center" />
{/if}
