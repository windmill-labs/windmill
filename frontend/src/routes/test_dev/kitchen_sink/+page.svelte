<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ClearableInput from '$lib/components/common/clearableInput/ClearableInput.svelte'
	import DateInput from '$lib/components/DateInput.svelte'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import ContextMenu from '$lib/components/common/contextmenu/ContextMenu.svelte'
	import { StickyNote } from 'lucide-svelte'
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
		Sun,
		X
	} from 'lucide-svelte'

	let darkMode: boolean = $state(false)
	let selectedToggle1: string = $state('option1')
	let selectedToggle2: string = $state('small')
	let selectedToggle3: string = $state('view')

	// Input states
	let basicTextValue: string = $state('')
	let placeholderTextValue: string = $state('')
	let errorTextValue: string = $state('Invalid input')
	let disabledTextValue: string = $state('Disabled value')
	let clearableValue: string = $state('Clearable text')
	let textAreaValue: string = $state('This is a text area\nwith multiple lines')
	let numberValue: number = $state(42)
	let dateValue: string = $state('')
	let dateTimeValue: string = $state('')

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

	// Context menu items
	const contextMenuItems = [
		{
			id: 'create-note',
			label: 'Create sticky note',
			icon: StickyNote,
			onClick: () => console.log('Create sticky note clicked')
		},
		{
			id: 'edit',
			label: 'Edit item',
			icon: Pen,
			onClick: () => console.log('Edit item clicked')
		},
		{
			id: 'divider-1',
			label: '',
			divider: true
		},
		{
			id: 'copy',
			label: 'Copy',
			icon: Copy,
			onClick: () => console.log('Copy clicked')
		},
		{
			id: 'share',
			label: 'Share',
			icon: Share,
			onClick: () => console.log('Share clicked')
		},
		{
			id: 'delete',
			label: 'Delete',
			icon: Trash,
			onClick: () => console.log('Delete clicked')
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
					<h4 class="text-xs font-semibold text-emphasis">Default</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">Accent</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">Accent Secondary</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">Subtle</h4>
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

	<!-- Destructive Buttons -->
	<div class="flex flex-col gap-6 mt-8">
		<h3 class="text-sm font-semibold text-emphasis">Destructive buttons</h3>
		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Destructive Default</h4>
					<p class="text-xs text-secondary">Delete actions (subtle)</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="default" size="sm" destructive>Delete Item</Button>
					<Button variant="default" size="xs" startIcon={{ icon: Trash }} destructive>Remove</Button
					>
					<Button variant="default" size="xs" disabled destructive>Disabled</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Destructive Accent</h4>
					<p class="text-xs text-secondary">Critical delete actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="sm" destructive>Delete Forever</Button>
					<Button variant="accent" size="xs" startIcon={{ icon: Trash }} destructive
						>Remove All</Button
					>
					<Button variant="accent" size="xs" disabled destructive>Disabled</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Destructive Accent Secondary</h4>
					<p class="text-xs text-secondary">Important delete actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent-secondary" size="sm" destructive>Archive</Button>
					<Button variant="accent-secondary" size="xs" startIcon={{ icon: Archive }} destructive
						>Archive Item</Button
					>
					<Button variant="accent-secondary" size="xs" disabled destructive>Disabled</Button>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Destructive Subtle</h4>
					<p class="text-xs text-secondary">Minimal delete actions</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="subtle" size="sm" destructive>Remove</Button>
					<Button variant="subtle" size="xs" startIcon={{ icon: X }} destructive>Clear</Button>
					<Button variant="subtle" size="xs" disabled destructive>Disabled</Button>
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
					<h4 class="text-xs font-semibold text-emphasis">Large (lg)</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">Medium (sm)</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">Small (xs)</h4>
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
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Default with dropdown</h4>
					<p class="text-xs text-secondary">Standard actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="default" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button
						variant="default"
						size="xs"
						dropdownItems={buttonDropdownItems}
						startIcon={{ icon: Settings }}>Options</Button
					>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Accent with dropdown</h4>
					<p class="text-xs text-secondary">Primary actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button
						variant="accent"
						size="xs"
						dropdownItems={buttonDropdownItems}
						startIcon={{ icon: Play }}>Run</Button
					>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Accent Secondary with dropdown</h4>
					<p class="text-xs text-secondary">Secondary actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent-secondary" size="sm" dropdownItems={buttonDropdownItems}
						>Actions</Button
					>
					<Button
						variant="accent-secondary"
						size="xs"
						dropdownItems={buttonDropdownItems}
						startIcon={{ icon: GitFork }}>Fork</Button
					>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Subtle with dropdown</h4>
					<p class="text-xs text-secondary">Utility actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="subtle" size="sm" dropdownItems={buttonDropdownItems}>Actions</Button>
					<Button
						variant="subtle"
						size="xs"
						dropdownItems={buttonDropdownItems}
						startIcon={{ icon: Eye }}>View</Button
					>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Destructive with dropdown</h4>
					<p class="text-xs text-secondary">Delete actions with menu</p>
				</div>
				<div class="flex flex-col space-y-2">
					<Button variant="accent" size="sm" dropdownItems={buttonDropdownItems} destructive
						>Delete</Button
					>
					<Button
						variant="default"
						size="xs"
						dropdownItems={buttonDropdownItems}
						startIcon={{ icon: Trash }}
						destructive>Remove</Button
					>
				</div>
			</div>
		</div>
	</div>

	<!-- TOGGLE SECTION HEADER -->
	<header class="flex flex-col gap-2 border-b border-gray-200 dark:border-gray-700 pb-4 mt-12">
		<h2 class="text-lg font-semibold text-emphasis">Toggle Buttons</h2>
		<p class="text-xs text-secondary"> Toggle button groups for selection and filtering. </p>
	</header>

	<!-- Toggle Examples -->
	<div class="flex flex-col gap-6">
		<h3 class="text-sm font-semibold text-emphasis">Toggle button groups</h3>

		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Basic Toggle</h4>
					<p class="text-xs text-secondary">Simple option selection</p>
				</div>
				<div class="flex flex-col space-y-2">
					<ToggleButtonGroup bind:selected={selectedToggle1}>
						{#snippet children({ item })}
							<ToggleButton value="option1" label="Option 1" {item} />
							<ToggleButton value="option2" label="Option 2" {item} />
							<ToggleButton value="option3" label="Option 3" {item} />
						{/snippet}
					</ToggleButtonGroup>
					<p class="text-2xs text-primary">Selected: {selectedToggle1}</p>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">With Icons</h4>
					<p class="text-xs text-secondary">Toggle with icon variants</p>
				</div>
				<div class="flex flex-col space-y-2">
					<ToggleButtonGroup bind:selected={selectedToggle2}>
						{#snippet children({ item })}
							<ToggleButton value="small" label="Small" icon={Search} {item} small />
							<ToggleButton value="medium" label="Medium" icon={Settings} {item} small />
							<ToggleButton value="large" label="Large" icon={Plus} {item} small />
						{/snippet}
					</ToggleButtonGroup>
					<p class="text-2xs text-primary">Selected: {selectedToggle2}</p>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Icon Only</h4>
					<p class="text-xs text-secondary">Compact icon-only toggles</p>
				</div>
				<div class="flex flex-col space-y-2">
					<ToggleButtonGroup bind:selected={selectedToggle3}>
						{#snippet children({ item })}
							<ToggleButton value="view" icon={Eye} {item} iconOnly tooltip="View mode" />
							<ToggleButton value="edit" icon={Pen} {item} iconOnly tooltip="Edit mode" />
							<ToggleButton value="share" icon={Share} {item} iconOnly tooltip="Share mode" />
						{/snippet}
					</ToggleButtonGroup>
					<p class="text-2xs text-primary">Selected: {selectedToggle3}</p>
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
					<h4 class="text-xs font-semibold text-emphasis">Standard Actions</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">With Links</h4>
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
					<h4 class="text-xs font-semibold text-emphasis">Mixed States</h4>
					<p class="text-xs text-secondary">Enabled and disabled</p>
				</div>
				<div class="flex justify-end">
					<DropdownV2 items={disabledItems} aiId="disabled-dropdown" />
				</div>
			</div>
		</div>
	</div>

	<!-- CONTEXT MENU SECTION HEADER -->
	<header class="flex flex-col gap-2 border-b border-gray-200 dark:border-gray-700 pb-4 mt-12">
		<h2 class="text-lg font-semibold text-emphasis">Context Menus</h2>
		<p class="text-xs text-secondary"> Right-click triggered menus with contextual actions. </p>
	</header>

	<!-- Context Menu Examples -->
	<div class="flex flex-col gap-6">
		<h3 class="text-sm font-semibold text-emphasis">Context menu tests</h3>

		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Basic Context Menu</h4>
					<p class="text-xs text-secondary">Right-click the area below</p>
				</div>
				<ContextMenu items={contextMenuItems}>
					<div
						class="w-full h-32 border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg flex items-center justify-center text-sm text-secondary bg-surface-tertiary cursor-pointer hover:border-accent hover:text-accent transition-colors"
					>
						Right-click me for context menu
					</div>
				</ContextMenu>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Text Context Menu</h4>
					<p class="text-xs text-secondary">Right-click the text below</p>
				</div>
				<ContextMenu items={contextMenuItems}>
					<div class="p-4 border rounded-lg bg-surface-tertiary">
						<p class="text-sm">
							Right-click this text to open the context menu. You can test various interactions
							here.
						</p>
					</div>
				</ContextMenu>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Button Context Menu</h4>
					<p class="text-xs text-secondary">Right-click the button below</p>
				</div>
				<ContextMenu items={contextMenuItems}>
					<Button variant="accent" size="sm">Right-click this button</Button>
				</ContextMenu>
			</div>
		</div>
	</div>

	<!-- INPUT SECTION HEADER -->
	<header class="flex flex-col gap-2 border-b border-gray-200 dark:border-gray-700 pb-4 mt-12">
		<h2 class="text-lg font-semibold text-emphasis">Input Components</h2>
		<p class="text-xs text-secondary">
			Text inputs, date pickers, and form controls with proper validation states.
		</p>
	</header>

	<!-- Basic Text Inputs -->
	<div class="flex flex-col gap-6">
		<h3 class="text-sm font-semibold text-emphasis">Text inputs</h3>

		<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Basic Text Input</h4>
					<p class="text-xs text-secondary">Standard text field</p>
				</div>
				<div class="space-y-2">
					<TextInput bind:value={basicTextValue} />
					<TextInput
						bind:value={placeholderTextValue}
						inputProps={{ placeholder: 'Enter text here...' }}
					/>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Input States</h4>
					<p class="text-xs text-secondary">Error and disabled states</p>
				</div>
				<div class="space-y-2">
					<TextInput bind:value={errorTextValue} error="This field has an error" />
					<TextInput bind:value={disabledTextValue} inputProps={{ disabled: true }} />
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Clearable Inputs</h4>
					<p class="text-xs text-secondary">Text inputs with clear button</p>
				</div>
				<div class="space-y-2">
					<ClearableInput bind:value={clearableValue} placeholder="Clearable text input" />
					<ClearableInput bind:value={numberValue} type="number" placeholder="Number input" />
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Text Area</h4>
					<p class="text-xs text-secondary">Multi-line text input</p>
				</div>
				<div class="space-y-2">
					<ClearableInput
						bind:value={textAreaValue}
						type="textarea"
						placeholder="Enter multiple lines..."
					/>
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">Date Input</h4>
					<p class="text-xs text-secondary">Date picker component</p>
				</div>
				<div class="space-y-2">
					<DateInput bind:value={dateValue} />
				</div>
			</div>

			<div
				class="flex flex-col space-y-4 p-6 border border-gray-200 dark:border-gray-700 rounded-lg bg-surface"
			>
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-emphasis">DateTime Input</h4>
					<p class="text-xs text-secondary">Date and time picker</p>
				</div>
				<div class="space-y-2">
					<DateTimeInput bind:value={dateTimeValue} />
				</div>
			</div>
		</div>
	</div>
</div>

<DarkModeObserver bind:darkMode />
