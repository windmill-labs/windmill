<script lang="ts">
	import Uptodate from './Uptodate.svelte'
	import Version from './Version.svelte'
	import DarkModeToggle from './sidebar/DarkModeToggle.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import { whitelabelNameStore } from '$lib/stores'
	import { capitalize } from '$lib/utils'

	interface Props {
		/** Off for the login page, which puts the mark and the instance name in the middle. */
		showBrand?: boolean
	}

	let { showBrand = true }: Props = $props()
</script>

<div class="absolute top-0 inset-x-0 flex items-center justify-between gap-2 px-4 py-2">
	<!-- The brand belongs to the instance, not to Windmill: a whitelabelled one keeps its own
		name and drops the mark, the same trade the centered logo used to make. -->
	<div class="flex items-center gap-2 text-base font-semibold text-emphasis">
		{#if showBrand}
			{#if $whitelabelNameStore}
				{capitalize($whitelabelNameStore)}
			{:else}
				<WindmillIcon height="28px" width="28px" />
				Windmill
			{/if}
		{/if}
	</div>

	<div class="flex flex-row gap-2 text-2xs text-gray-800 italic">
		<DarkModeToggle forcedDarkMode={false} />

		<div class="font-mono flex-col flex p-2 justify-center">
			<Version />
			<Uptodate />
		</div>
	</div>
</div>
