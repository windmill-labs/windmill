<script lang="ts">
	import { Moon, Sun } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let darkMode: boolean = document.documentElement.classList.contains('dark')

	export let forcedDarkMode: boolean = true
</script>

<button
	class={twMerge(
		'text-2xs text-white m-2 p-2 rounded-lg flex flex-row gap-2 justify-center hover:bg-gray-600',
		forcedDarkMode ? 'text-white hover:bg-gray-600' : 'text-primary hover:bg-surface-hover'
	)}
	on:click={() => {
		if (!document.documentElement.classList.contains('dark')) {
			document.documentElement.classList.add('dark')
			window.localStorage.setItem('dark-mode', 'dark')
			darkMode = true
		} else {
			document.documentElement.classList.remove('dark')
			window.localStorage.setItem('dark-mode', 'light')
			darkMode = false
		}
	}}
>
	{#if darkMode}
		<Sun class="w-4 h-4" />
	{:else}
		<Moon class="w-4 h-4" />
	{/if}
</button>
