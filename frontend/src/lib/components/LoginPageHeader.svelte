<script>
	import Uptodate from './Uptodate.svelte'
	import Version from './Version.svelte'
	import DarkModeToggle from './sidebar/DarkModeToggle.svelte'
</script>

<div class="absolute top-0 right-0 text-2xs text-gray-800 italic px-3 py-1">
	<div class="flex flex-row gap-2">
		<DarkModeToggle forcedDarkMode={false} />

		<div class="font-mono flex-col flex p-2 justify-center">
			<Version />
			<Uptodate />
		</div>
	</div>
</div>
