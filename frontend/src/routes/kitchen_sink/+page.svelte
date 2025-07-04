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
	<Tab value="button">Buttons</Tab>

	{#snippet content()}
		<TabContent value="button" class="p-4 flex gap-4 flex-col ">
			<div class="font-bold text-md">Contained buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button>Lorem</Button>
				<Button disabled>Lorem</Button>

				<Button color="dark" loading>Lorem</Button>
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
				<Button variant="border">Lorem</Button>
				<Button variant="border" color="dark">Lorem</Button>
				<Button variant="border" color="gray">Lorem</Button>
				<Button variant="border" color="green">Lorem</Button>
				<Button variant="border" color="light">Lorem</Button>
				<Button variant="border" color="none">Lorem</Button>
				<Button variant="border" color="red">Lorem</Button>
			</div>
			<div class="font-bold text-md">Dropdown buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button {dropdownItems}>Lorem</Button>
				<Button {dropdownItems} color="dark">Lorem</Button>
				<Button {dropdownItems} color="gray">Lorem</Button>
				<Button {dropdownItems} color="green">Lorem</Button>
				<Button {dropdownItems} color="light">Lorem</Button>
				<Button {dropdownItems} color="none">Lorem</Button>
				<Button {dropdownItems} color="red">Lorem</Button>
			</div>
			<div class="font-bold text-md">Dropdown buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button variant="border" {dropdownItems}>Lorem</Button>
				<Button variant="border" {dropdownItems} color="dark">Lorem</Button>
				<Button variant="border" {dropdownItems} color="gray">Lorem</Button>
				<Button variant="border" {dropdownItems} color="green">Lorem</Button>
				<Button variant="border" {dropdownItems} color="light">Lorem</Button>
				<Button variant="border" {dropdownItems} color="none">Lorem</Button>
				<Button variant="border" {dropdownItems} color="red">Lorem</Button>
			</div>
		</TabContent>
	{/snippet}
</Tabs>
