<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let open: boolean = false
	export let z = 'z-30'

	const dispatch = createEventDispatcher()

	export function closeDrawer(): void {
		document.body.style.overflow = 'auto'

		open = false
		dispatch('close')
	}

	export function openDrawer(): void {
		document.body.style.overflow = 'hidden'
		open = true
		dispatch('open')
	}

	function handleKeyUp(event: KeyboardEvent): void {
		const key = event.key
		if (key === 'Escape' || key === 'Esc') {
			if (open) {
				event.preventDefault()
				closeDrawer()
			}
		}
	}
</script>

<svelte:window on:keyup={handleKeyUp} />

{#if open}
	<div class="blurred-background" />

	<div class="fixed top-0 w-screen h-screen {z}">
		<div
			class="fixed right-0 top-0 flex flex-col w-3/4 sm:w-2/3 lg:w-1/2 h-screen border border-gray-300 shadow-xl"
		>
			{#if open}
				<div class="flex flex-row justify-between p-2 bg-white border-b border-gray-200">
					<button
						on:click={() => {
							open = false
							closeDrawer()
						}}
					>
						<svg
							class="w-6 h-6"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
							xmlns="http://www.w3.org/2000/svg"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</button>
					<p class="font-semibold text-gray-800"><slot name="title" /></p>
					<div />
				</div>
				<div class="flex flex-col bg-gray-50 pt-3 px-6 grow overflow-y-auto">
					<slot name="content" />
				</div>
				<div class="flex flex-col bg-white border-gray-200 p-2">
					<div class="flex flex-row justify-between p-2">
						<button
							on:click={() => {
								closeDrawer()
							}}
						>
							<svg
								class="w-6 h-6"
								fill="none"
								stroke="currentColor"
								viewBox="0 0 24 24"
								xmlns="http://www.w3.org/2000/svg"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								/>
							</svg>
						</button>
						<span class="mr-4"><slot name="submission">&nbsp;</slot></span>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style lang="postcss">
	.blurred-background {
		/* @apply absolute sm:top-6 lg:top-8 left-28 sm:left-40 md:left-48; */ /* If we wanted to make the navbars visible */
		@apply fixed  top-0 left-0;
		@apply bg-gray-400 opacity-75;
		@apply w-screen;
		@apply h-screen;
		z-index: 10;
	}
</style>
