<script lang="ts">
	export let notificationCount = 0
	export let notificationLimit: number | undefined = undefined
</script>

{#if notificationCount > 0}
	<div
		class="bg-red-500 text-white text-[0.6rem] rounded-md w-5 h-5 flex items-center justify-center"
	>
		{#if notificationLimit && notificationCount > notificationLimit}
			{`${notificationLimit}+`}
		{:else}
			{notificationCount}
		{/if}
	</div>
{/if}
