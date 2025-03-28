<script lang="ts">
	import { Pin } from 'lucide-svelte'
	import { base } from '$lib/base'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Info } from 'lucide-svelte'

	let darkMode = true

	function openFullscreen(event: MouseEvent) {
		const img = event.target as HTMLImageElement
		if (img.requestFullscreen) {
			img.requestFullscreen({
				navigationUI: 'auto'
			})
		}
	}
</script>

<DarkModeObserver bind:darkMode />

<div class="p-1.5">
	<!-- Header Banner -->
	<div
		class="bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20
                text-blue-700 dark:text-blue-300 rounded-md px-3 py-2 flex items-center justify-between mb-3"
	>
		<div class="flex items-center gap-2">
			<div
				class="bg-blue-600 dark:bg-blue-400 px-1.5 py-0.5 text-white text-xs font-bold tracking-wide rounded shadow-sm"
				>NEW</div
			>
			<span class="font-normal text-sm">Mock has evolved into</span>
			<span class="font-semibold flex items-center gap-0.5 text-sm">
				<Pin size="14" strokeWidth="2.5" />
				PIN
			</span>
		</div>
		<div class="text-xs bg-white/60 dark:bg-slate-800/60 py-0.5 px-2 rounded">
			<span class="font-medium">Find it in:</span>
			<span class="font-semibold">"Test this step"</span> tab
		</div>
	</div>

	<!-- Main Content -->
	<div class="text-xs">
		<!-- Steps Header -->
		<span class="flex items-center mb-2 text-slate-500"> How to use the PIN feature: </span>

		<!-- Steps Container -->
		<div class="grid grid-cols-2 gap-3">
			<!-- Step 1 -->
			<div class="flex items-start gap-2 group">
				<div
					class="bg-surface-secondary text-secondary rounded-full w-5 h-5 flex items-center justify-center text-xs font-medium flex-shrink-0 -mt-0.5"
					>1</div
				>
				<div class="w-full">
					<div class="font-medium mb-1.5">Pick a result from history</div>
					<div
						class="border rounded-md p-1 bg-white dark:bg-slate-800 h-56 overflow-hidden shadow-sm group-hover:shadow transition-shadow"
					>
						<!--svelte-ignore a11y-click-events-have-key-events-->
						<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
						<img
							class="w-full h-full object-contain opacity-70 hover:opacity-100 cursor-pointer"
							src={darkMode ? `${base}/pin-history-dark.png` : `${base}/pin-history.png`}
							alt="History picker"
							on:click={openFullscreen}
						/>
					</div>
				</div>
			</div>

			<!-- Step 2 -->
			<div class="flex items-start gap-2 group">
				<div
					class="bg-surface-secondary text-secondary rounded-full w-5 h-5 flex items-center justify-center text-xs font-medium flex-shrink-0 -mt-0.5"
					>2</div
				>
				<div class="w-full">
					<div class="font-medium mb-1.5">Pin it as a fixed output</div>
					<div
						class="border rounded-md p-1 bg-white dark:bg-slate-800 h-56 overflow-hidden shadow-sm group-hover:shadow transition-shadow"
					>
						<!--svelte-ignore a11y-click-events-have-key-events-->
						<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
						<img
							class="w-full h-full object-contain opacity-70 hover:opacity-100 cursor-pointer"
							src={darkMode ? `${base}/pin-pin-dark.png` : `${base}/pin-pin.png`}
							alt="Pin action"
							on:click={openFullscreen}
						/>
					</div>
				</div>
			</div>

			<!-- Step 3 -->
			<div class="flex items-start gap-2 group">
				<div
					class="bg-surface-secondary text-secondary rounded-full w-5 h-5 flex items-center justify-center text-xs font-medium flex-shrink-0 -mt-0.5"
					><Info size="14" /></div
				>
				<div class="w-full">
					<div class="font-medium mb-1.5">The last pin can be recovered from history</div>
					<div
						class="border rounded-md p-1 bg-white dark:bg-slate-800 h-56 overflow-hidden shadow-sm group-hover:shadow transition-shadow"
					>
						<!--svelte-ignore a11y-click-events-have-key-events-->
						<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
						<img
							class="w-full h-full object-contain opacity-70 hover:opacity-100 cursor-pointer"
							src={darkMode ? `${base}/pin-restore-dark.png` : `${base}/pin-restore.png`}
							alt="Recover pins"
							on:click={openFullscreen}
						/>
					</div>
				</div>
			</div>

			<!-- Step 4 -->
			<div class="flex items-start gap-2 group">
				<div
					class="bg-surface-secondary text-secondary rounded-full w-5 h-5 flex items-center justify-center text-xs font-medium flex-shrink-0 -mt-0.5"
				>
					<Info size="14" />
				</div>
				<div class="w-full">
					<div class="font-medium mb-1.5">All of this can be done from the flow view</div>
					<div
						class="border rounded-md p-1 bg-white dark:bg-slate-800 h-56 overflow-hidden shadow-sm group-hover:shadow transition-shadow"
					>
						<!--svelte-ignore a11y-click-events-have-key-events-->
						<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
						<img
							class="w-full h-full object-contain opacity-70 hover:opacity-100 cursor-pointer"
							src={darkMode ? `${base}/pin-flow-view-dark.png` : `${base}/pin-flow-view.png`}
							alt="Flow view pinning"
							on:click={openFullscreen}
						/>
					</div>
				</div>
			</div>
		</div>
	</div>
</div>
