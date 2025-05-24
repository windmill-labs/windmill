<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'
	import { twMerge } from 'tailwind-merge'
	import { userStore } from '$lib/stores'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { TriggerContext } from '../triggers'
	import { Calendar } from 'lucide-svelte'

	type MainButton = {
		label: string
		href: string
		buttonProps: ButtonProps
	}

	type ButtonProps = any
	type MenuItemButton = {
		label: string
		Icon: any
		onclick: () => void
		color?: 'red'
	}

	const { triggersCount, selectedTrigger } = getContext<TriggerContext>('TriggerContext')

	export let mainButtons: MainButton[] = []
	export let menuItems: MenuItemButton[] = []
	export let title: string
	export let tag: string | undefined

	export let errorHandlerKind: 'flow' | 'script'
	export let scriptOrFlowPath: string
	export let errorHandlerMuted: boolean | undefined

	const dispatch = createEventDispatcher()
</script>

<div class="border-b p-2 shadow-md">
	<div class="mx-auto">
		<div class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center">
			<div class="grow px-2 inline-flex items-center gap-4 min-w-0">
				<div
					class={twMerge(
						'text-lg min-w-24 font-bold truncate',
						$userStore?.operator ? 'pl-10' : ''
					)}
				>
					{title}
				</div>{#if tag}
					<Badge>tag: {tag}</Badge>
				{/if}
				<slot />
				{#if $triggersCount?.primary_schedule}
					<Button
						btnClasses="inline-flex"
						startIcon={{ icon: Calendar }}
						variant="contained"
						color="light"
						size="xs"
						on:click={() => {
							$selectedTrigger = 'schedules'
							dispatch('triggerDetail')
						}}
					>
						{$triggersCount?.primary_schedule?.schedule ?? ''}
					</Button>
				{/if}
				<slot name="trigger-badges" />
			</div>
			<div class="flex gap-1 md:gap-2 items-center">
				{#if menuItems.length > 0}
					{#key menuItems}
						<DropdownV2
							items={menuItems.map((item) => ({
								displayName: item.label,
								icon: item.Icon,
								action: item.onclick,
								type: item.color === 'red' ? 'delete' : 'action'
							}))}
							placement="bottom-end"
						/>
					{/key}
				{/if}
				<ErrorHandlerToggleButton
					kind={errorHandlerKind}
					{scriptOrFlowPath}
					bind:errorHandlerMuted
				/>
				{#each mainButtons as btn}
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						on:click={btn.buttonProps.onClick}
						btnClasses="hidden md:flex items-center gap-1"
					>
						{btn.label}
					</Button>
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						on:click={btn.buttonProps.onClick}
						iconOnly
						btnClasses="flex md:hidden items-center gap-1"
					>
						{btn.label}
					</Button>
				{/each}
			</div>
		</div>
	</div>
</div>
