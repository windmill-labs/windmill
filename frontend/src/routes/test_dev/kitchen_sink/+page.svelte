<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import {
		Pen,
		GitFork,
		Trash,
		Eye,
		Share,
		Copy,
		Archive,
		Settings,
		Earth,
		Database,
		Code,
		Play,
		Plus,
		Save,
		Search,
		Moon,
		Sun
	} from 'lucide-svelte'

	let darkMode: boolean = $state(false)

	function toggleTheme() {
		if (!document.documentElement.classList.contains('dark')) {
			document.documentElement.classList.add('dark')
			window.localStorage.setItem('dark-mode', 'dark')
		} else {
			document.documentElement.classList.remove('dark')
			window.localStorage.setItem('dark-mode', 'light')
		}
	}

	// Basic dropdown items
	const basicItems = [
		{
			displayName: 'Edit',
			icon: Pen,
			action: () => console.log('Edit clicked')
		},
		{
			displayName: 'Duplicate',
			icon: GitFork,
			action: () => console.log('Duplicate clicked')
		},
		{
			displayName: 'Share',
			icon: Share,
			action: () => console.log('Share clicked')
		},
		{
			displayName: 'Archive',
			icon: Archive,
			action: () => console.log('Archive clicked'),
			type: 'delete' as const
		}
	]

	// Dropdown with disabled items
	const disabledItems = [
		{
			displayName: 'Available Action',
			icon: Eye,
			action: () => console.log('Available action')
		},
		{
			displayName: 'Disabled Action',
			icon: Settings,
			action: () => console.log('This should not fire'),
			disabled: true
		},
		{
			displayName: 'Another Available',
			icon: Copy,
			action: () => console.log('Another available action')
		},
		{
			displayName: 'Disabled Delete',
			icon: Trash,
			action: () => console.log('This should not fire'),
			disabled: true,
			type: 'delete' as const
		}
	]

	// Dropdown with external links
	const linkItems = [
		{
			displayName: 'Internal Link',
			icon: Code,
			href: '/scripts'
		},
		{
			displayName: 'External Link',
			icon: Earth,
			href: 'https://windmill.dev',
			hrefTarget: '_blank' as const
		},
		{
			displayName: 'Documentation',
			icon: Database,
			href: 'https://docs.windmill.dev',
			hrefTarget: '_blank' as const
		}
	]

	// Button dropdown items
	const buttonDropdownItems = [
		{
			label: 'Edit',
			icon: Pen,
			onClick: () => console.log('Edit clicked')
		},
		{
			label: 'Duplicate',
			icon: GitFork,
			onClick: () => console.log('Duplicate clicked')
		},
		{
			label: 'Archive',
			icon: Archive,
			onClick: () => console.log('Archive clicked')
		},
		{
			label: 'Delete',
			icon: Trash,
			onClick: () => console.log('Delete clicked'),
			disabled: true
		}
	]
</script>

<div class="p-8 flex flex-col gap-8 max-w-6xl mx-auto bg-surface-secondary min-h-screen pb-32">
	<header class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<h1 class="text-2xl font-semibold text-emphasis">Design system kitchen sink</h1>
			<Button
				variant="subtle"
				size="xs"
				startIcon={{ icon: darkMode ? Sun : Moon }}
				onclick={toggleTheme}
				title={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
			>
				{darkMode ? 'Light mode' : 'Dark mode'}
			</Button>
		</div>
		<p class="text-xs text-secondary max-w-3xl">
			Showcase of Button and Dropdown components following Windmill's design system guidelines.
		</p>
	</header>

	<!-- BUTTON SECTION HEADER -->
	<header class="flex flex-col gap-2 border-b border-gray-200 dark:border-gray-700 pb-4">
		<h2 class="text-lg font-semibold text-emphasis">Buttons</h2>
		<p class="text-xs text-secondary">
			Design system button variants following semantic color guidelines.
		</p>
	</header>

	<!-- Button Variants -->
	<div class="flex flex-col gap-6">
		<h3 class="text-sm font-semibold text-emphasis">Button variants</h3>

		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Default</h4>
					<p class="text-xs text-secondary">Standard actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="default" size="sm">Default Button</Button>
					<Button variant="default" size="xs" startIcon={{ icon: Pen }}>With Icon</Button>
					<Button variant="default" size="xs" disabled>Disabled</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Accent</h4>
					<p class="text-xs text-secondary">Primary actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="sm">Accent Button</Button>
					<Button variant="accent" size="xs" startIcon={{ icon: Play }}>Run Script</Button>
					<Button variant="accent" size="xs" disabled>Disabled</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Accent Secondary</h4>
					<p class="text-xs text-secondary">Secondary actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent-secondary" size="sm">Secondary</Button>
					<Button variant="accent-secondary" size="xs" startIcon={{ icon: GitFork }}>Fork</Button>
					<Button variant="accent-secondary" size="xs" disabled>Disabled</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Subtle</h4>
					<p class="text-xs text-secondary">Utility actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="subtle" size="sm">Subtle Button</Button>
					<Button variant="subtle" size="xs" startIcon={{ icon: Eye }}>View</Button>
					<Button variant="subtle" size="xs" disabled>Disabled</Button>
				</div>
			</div>
		</div>
	</div>

	<!-- Button Sizes -->
	<div class="flex flex-col gap-6 mt-8">
		<h3 class="text-sm font-semibold text-emphasis">Button sizes</h3>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Large (lg)</h4>
					<p class="text-xs text-secondary">Prominent actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="lg">Large Button</Button>
					<Button variant="default" size="lg" startIcon={{ icon: Plus }}>Add Item</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Medium (sm)</h4>
					<p class="text-xs text-secondary">Standard size</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="sm">Medium Button</Button>
					<Button variant="default" size="sm" startIcon={{ icon: Save }}>Save</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Small (xs)</h4>
					<p class="text-xs text-secondary">Compact interfaces</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="xs">Small Button</Button>
					<Button variant="default" size="xs" startIcon={{ icon: Search }}>Search</Button>
				</div>
			</div>
		</div>
	</div>

	<!-- Buttons with dropdown -->
	<div class="flex flex-col gap-6 mt-8">
		<h3 class="text-sm font-semibold text-emphasis">Buttons with dropdown</h3>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
			<div class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface">
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Default with dropdown</h4>
					<p class="text-xs text-secondary">Standard actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="default" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button variant="default" size="xs" dropdownItems={buttonDropdownItems} startIcon={{ icon: Settings }}>Options</Button>
				</div>
			</div>

			<div class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface">
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Accent with dropdown</h4>
					<p class="text-xs text-secondary">Primary actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button variant="accent" size="xs" dropdownItems={buttonDropdownItems} startIcon={{ icon: Play }}>Run</Button>
				</div>
			</div>

			<div class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface">
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Accent Secondary with dropdown</h4>
					<p class="text-xs text-secondary">Secondary actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent-secondary" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button variant="accent-secondary" size="xs" dropdownItems={buttonDropdownItems} startIcon={{ icon: GitFork }}>Fork</Button>
				</div>
			</div>

			<div class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface">
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Subtle with dropdown</h4>
					<p class="text-xs text-secondary">Utility actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="subtle" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button variant="subtle" size="xs" dropdownItems={buttonDropdownItems} startIcon={{ icon: Eye }}>View</Button>
				</div>
			</div>
		</div>
	</div>

	<!-- DROPDOWN SECTION HEADER -->
	<header class="flex flex-col gap-2 border-b border-gray-200 dark:border-gray-700 pb-4 mt-12">
		<h2 class="text-lg font-semibold text-emphasis">Dropdowns</h2>
		<p class="text-xs text-secondary">
			Contextual menu components with consistent styling and accessibility.
		</p>
	</header>

	<!-- Dropdown Examples -->
	<div class="flex flex-col gap-6">
		<h3 class="text-sm font-semibold text-emphasis">Dropdown types</h3>

		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Standard Actions</h4>
					<p class="text-xs text-secondary">Common operations</p>
				</div>
				<div class="flex justify-end">
					<DropdownV2 items={basicItems} aiId="basic-dropdown" />
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">With Links</h4>
					<p class="text-xs text-secondary">Navigation items</p>
				</div>
				<div class="flex justify-end">
					<DropdownV2 items={linkItems} aiId="link-dropdown" />
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-medium text-emphasis">Mixed States</h4>
					<p class="text-xs text-secondary">Enabled and disabled</p>
				</div>
				<div class="flex justify-end">
					<DropdownV2 items={disabledItems} aiId="disabled-dropdown" />
				</div>
			</div>
		</div>
	</div>
</div>

<DarkModeObserver bind:darkMode />
