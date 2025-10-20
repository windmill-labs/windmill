<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import { Globe } from 'lucide-svelte'

	let tab = $state('button')
	let dropdownItems = [
		{
			label: 'Lorem ipsum',
			onClick: () => {}
		}
	]
</script>

<DarkModeToggle forcedDarkMode={false} />

<Tabs bind:selected={tab}>
	<Tab value="button" label="Buttons" />

	{#snippet content()}
		<TabContent value="button" class="p-4 flex gap-4 flex-col ">
			<div class="font-bold text-md">Contained buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button>Lorem</Button>
				<Button disabled>Lorem</Button>

				<Button variant="accent" loading>Lorem</Button>
				<Button
					color="gray"
					startIcon={{
						icon: Globe
					}}
				>
					Lorem
				</Button>
				<Button color="green">Lorem</Button>
				<Button color="light">Lorem</Button>
				<Button color="none">Lorem</Button>
				<Button color="red">Lorem</Button>
			</div>
			<div class="font-bold text-md">Border buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
			</div>
			<div class="font-bold text-md">Dropdown buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button {dropdownItems}>Lorem</Button>
				<Button {dropdownItems} variant="accent">Lorem</Button>
				<Button {dropdownItems} color="gray">Lorem</Button>
				<Button {dropdownItems} color="green">Lorem</Button>
				<Button {dropdownItems} color="light">Lorem</Button>
				<Button {dropdownItems} color="none">Lorem</Button>
				<Button {dropdownItems} color="red">Lorem</Button>
			</div>
			<div class="font-bold text-md">Dropdown buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
			</div>
		</TabContent>
	{/snippet}
</Tabs>
