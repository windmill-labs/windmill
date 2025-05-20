<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { AlertTriangle, CornerDownLeft, Loader2, RefreshCcw } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	type Props = {
		title: string
		confirmationText: string
		keyListen?: boolean
		loading?: boolean
		open?: boolean
		type?: 'danger' | 'reload'
		modalId?: string
		wrapperClass?: string
	}

	const {
		title,
		confirmationText,
		keyListen = true,
		loading = false,
		open = false,
		type: _type,
		modalId = undefined,
		wrapperClass = ''
	}: Props = $props()
	const type = $derived(_type ?? 'danger')

	const dispatch = createEventDispatcher()

	function onKeyDown(event: KeyboardEvent) {
		if (open && keyListen) {
			event.stopPropagation()
			event.preventDefault()
			switch (event.key) {
				case 'Enter':
					dispatch('confirmed')
					break
				case 'Escape':
					dispatch('canceled')
					break
			}
		}
	}
	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}

	const theme = {
		danger: {
			Icon: AlertTriangle,
			color: 'red',
			classes: {
				icon: 'text-red-500 dark:text-red-400',
				iconWrapper: 'bg-red-100 dark:bg-red-800/50'
			}
		},

		reload: {
			Icon: RefreshCcw,
			color: 'dark',
			classes: {
				icon: 'text-blue-500 dark:text-blue-400',
				iconWrapper: 'bg-blue-100 dark:bg-blue-800/50'
			}
		}
	} satisfies { [type in typeof type]: any }
	const Icon = $derived(theme[type].Icon ?? AlertTriangle)
</script>

<svelte:window on:keydown|capture={onKeyDown} />

{#if open}
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		transition:fadeFast|local
		class={twMerge('absolute top-0 bottom-0 left-0 right-0 z-[5000]', wrapperClass)}
		role="dialog"
		id={modalId}
		onclick={(e) => {
			e.stopPropagation()
		}}
	>
		<div
			class={classNames(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
			)}
		></div>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class={classNames(
						'relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
						open
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
				>
					<div class="flex">
						<div
							class={`flex h-12 w-12 items-center justify-center rounded-full ${theme[type].classes.iconWrapper}`}
						>
							<Icon class={theme[type].classes.icon} />
						</div>
						<div class="ml-4 text-left flex-1">
							<h3 class="text-lg font-medium text-primary">
								{title}
							</h3>
							<div class="mt-2 text-sm text-secondary">
								<!-- svelte-ignore slot_element_deprecated -->
								<slot />
							</div>
						</div>
					</div>
					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						<Button
							disabled={loading}
							on:click={() => dispatch('confirmed')}
							color={theme[type].color}
							size="sm"
							shortCut={{ Icon: CornerDownLeft, hide: !keyListen, withoutModifier: true }}
						>
							{#if loading}
								<Loader2 class="animate-spin" />
							{/if}
							<span class="min-w-20">{confirmationText} </span>
						</Button>
						<Button
							disabled={loading}
							on:click={() => dispatch('canceled')}
							color="light"
							size="sm"
							shortCut={{ key: 'Esc', hide: !keyListen, withoutModifier: true }}
						>
							Cancel
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
