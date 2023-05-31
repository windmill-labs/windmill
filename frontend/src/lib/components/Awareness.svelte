<script lang="ts">
	import { awarenessStore, userStore } from '$lib/stores'
	import { BROWSER } from 'esm-env'

	let url = BROWSER ? window.location.pathname : ''
</script>

<div class="isolate flex -space-x-1 overflow-hidden w-20">
	{#each Object.entries($awarenessStore ?? {}).filter(([a, b]) => b == url && a != $userStore?.username) as [user, _]}
		<span
			class="inline-flex h-6 w-6 items-center justify-center rounded-full ring-2 ring-white bg-gray-600"
			title={user}
		>
			<span class="text-sm font-medium leading-none text-white"
				>{user.substring(0, 2).toLocaleUpperCase()}</span
			>
		</span>
	{/each}
</div>
