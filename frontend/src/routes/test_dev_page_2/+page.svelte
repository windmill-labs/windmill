<script>
	import { ChevronDown, History, Pin } from 'lucide-svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button } from '$lib/components/common'

	let isCardExpanded = false
	const NODE_HEIGHT = '34px'

	const jsonData = {
		name: 'John Doe',
		age: 30,
		city: 'New York'
	}

	const toggleCard = () => {
		isCardExpanded = !isCardExpanded
	}
</script>

<div class="h-screen w-full bg-surface-secondary p-4 flex items-center justify-center">
	<!-- Node -->
	<div class="relative shadow-md" style="--node-height: {NODE_HEIGHT}">
		<div
			class="absolute top-0 w-[275px] h-[var(--node-height)] bg-surface rounded-sm flex items-center justify-center z-10"
		>
			<span class="font-medium">Flow Node</span>
		</div>

		<!-- Expandable Card -->
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			class="absolute w-[275px] flex flex-col items-center justify-between bg-transparent top-0 rounded-sm shadow-lg transition-all duration-300 cursor-pointer px-1"
			style="padding-top: calc(var(--node-height)"
			class:h-[200px]={isCardExpanded}
			class:h-[44px]={!isCardExpanded}
			on:click={toggleCard}
		>
			{#if isCardExpanded}
				<div class="grow min-h-0 w-full flex flex-col overflow-hidden mt-1 gap-1">
					<div class="flex flex-row items-center justify-between">
						<div class="flex flex-row items-center gap-0.5">
							<Button
								color="light"
								size="xs2"
								variant="contained"
								btnClasses="bg-transparent"
								startIcon={{ icon: History }}
								iconOnly
							/>
							<Button
								color="light"
								size="xs2"
								variant="contained"
								btnClasses="bg-transparent"
								startIcon={{ icon: Pin }}
								iconOnly
							/>
						</div>
						<Toggle
							size="2xs"
							options={{
								right: 'JSON',
								rightTooltip:
									'Arguments can be edited either using the wizard, or by editing their JSON Schema.'
							}}
							textClass="text-2xs"
						/>
					</div>
					<div class="grow min-h-0 p-2 bg-surface rounded-sm">
						<ObjectViewer json={jsonData} />
					</div>
				</div>
			{/if}
			<div
				class="grow-0 w-3 h-3 transition-transform duration-100 center-center flex items-center justify-center"
				class:rotate-180={isCardExpanded}
			>
				<ChevronDown size={14} class="h-fit" />
			</div>
		</div>
	</div>
</div>

<style>
	/* Add any additional custom styles here if needed */
</style>
