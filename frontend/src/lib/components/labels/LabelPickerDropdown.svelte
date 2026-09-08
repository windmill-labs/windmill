<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import GenericDropdown from '$lib/components/select/GenericDropdown.svelte'
	import {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '$lib/components/text_input/TextInput.svelte'
	import { clickOutside } from '$lib/utils'
	import type { Label } from '$lib/gen'
	import { Pen, Plus } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import LabelForm from './LabelForm.svelte'
	import { LABEL_COLOR_SWATCHES } from './labelColors'

	interface Props {
		/** The workspace the labels belong to — not necessarily the one being navigated. */
		workspace: string | undefined
		/** Every label of the workspace, coloured or not. */
		labels: Label[]
		/** Names already on the item, filtered out of the choices. */
		taken: string[]
		open: boolean
		/** A label was chosen or created — the caller attaches it to its item. */
		onPick: (name: string) => void
		/** The registry changed, so the caller can refresh its copy of `labels`. */
		onLabelsChanged?: () => void
		inputClass?: string
	}

	let {
		workspace,
		labels,
		taken,
		open = $bindable(),
		onPick,
		onLabelsChanged,
		inputClass = ''
	}: Props = $props()

	type Page = { kind: 'list' } | { kind: 'form'; existing?: Label; initialName: string }

	let page = $state<Page>({ kind: 'list' })
	let filterText = $state('')
	let inputEl: HTMLInputElement | undefined = $state()
	let listHeight = $state(0)
	let formHeight = $state(0)

	let choices = $derived(
		labels.filter(
			(l) =>
				!taken.includes(l.name) && l.name.toLowerCase().includes(filterText.trim().toLowerCase())
		)
	)
	let trimmedFilter = $derived(filterText.trim())
	let onForm = $derived(page.kind === 'form')

	// The two pages are stacked, so the frame has to be told which one's height to
	// take; GenericDropdown measures this element every frame and animates its own
	// box to match, which is what makes the page change grow and shrink smoothly.
	let frameHeight = $derived(onForm ? formHeight : listHeight)

	export function reset() {
		page = { kind: 'list' }
		filterText = ''
	}

	export function focus() {
		inputEl?.focus()
	}

	function openForm(existing?: Label) {
		page = { kind: 'form', existing, initialName: existing ? '' : trimmedFilter }
	}

	function backToList() {
		page = { kind: 'list' }
		inputEl?.focus()
	}

	// Creating a label from the picker means the user wants it on the item, so the
	// save attaches it. Editing one only recolours it — the pen was clicked to fix a
	// colour, not to add the label — so it goes back to the list.
	function onSaved(name: string) {
		const wasCreating = page.kind === 'form' && page.existing == undefined
		onLabelsChanged?.()
		if (wasCreating) {
			onPick(name)
			open = false
		} else {
			backToList()
		}
	}

	function pick(name: string) {
		onPick(name)
		open = false
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault()
			if (onForm) {
				backToList()
			} else {
				open = false
			}
		} else if (e.key === 'Enter' && !onForm) {
			e.preventDefault()
			if (choices.length > 0) {
				pick(choices[0].name)
			} else if (trimmedFilter) {
				openForm()
			}
		}
	}
</script>

<div class="relative" use:clickOutside={{ onClickOutside: () => (open = false) }}>
	<!-- svelte-ignore a11y_autofocus -->
	<input
		bind:this={inputEl}
		bind:value={filterText}
		autofocus
		autocomplete="off"
		type="text"
		placeholder="label"
		onkeydown={onKeydown}
		onpointerdown={() => (open = true)}
		class={twMerge(
			inputBaseClass,
			inputSizeClasses.sm,
			inputBorderClass({ forceFocus: open }),
			'w-full',
			inputClass
		)}
	/>
</div>

<GenericDropdown
	{open}
	listAutoWidth={false}
	getInputRect={inputEl && (() => inputEl!.getBoundingClientRect())}
	innerClass="w-64"
	maxHeight={400}
>
	<!-- Both pages stay mounted so the slide has something to slide; the frame clips
	     them and only the active one contributes its height. -->
	<div
		class="relative overflow-hidden transition-[height] duration-200 ease-out"
		style="height: {frameHeight}px"
	>
		<div
			bind:clientHeight={listHeight}
			class="absolute inset-x-0 top-0 transition-transform duration-200 ease-out"
			style="transform: translateX({onForm ? '-100%' : '0'})"
			aria-hidden={onForm}
		>
			<ul class="flex flex-col max-h-56 overflow-y-auto">
				{#each choices as choice (choice.name)}
					<li class="flex flex-row items-center gap-1 pr-1 hover:bg-surface-hover">
						<button
							class="flex flex-row items-center gap-2 min-w-0 flex-1 py-2 pl-4 text-left text-xs text-primary"
							onclick={() => pick(choice.name)}
						>
							<span
								class="block w-2 h-2 rounded-full shrink-0 {LABEL_COLOR_SWATCHES[
									choice.color ?? 'blue'
								]}"
							></span>
							<span class="truncate">{choice.name}</span>
						</button>
						<Button
							variant="subtle"
							unifiedSize="2xs"
							startIcon={{ icon: Pen }}
							iconOnly
							aria-label="Edit label {choice.name}"
							onClick={() => openForm(choice)}
						/>
					</li>
				{/each}
				{#if choices.length === 0}
					<li class="py-6 px-4 text-center text-xs text-secondary">
						{trimmedFilter ? 'No matching label' : 'No other label'}
					</li>
				{/if}
			</ul>
			<div class="flex border-t">
				<Button
					variant="default"
					unifiedSize="sm"
					wrapperClasses="flex-1"
					btnClasses="rounded-none border-0"
					startIcon={{ icon: Plus }}
					onClick={() => openForm()}
				>
					New label
				</Button>
			</div>
		</div>
		<div
			bind:clientHeight={formHeight}
			class="absolute inset-x-0 top-0 transition-transform duration-200 ease-out"
			style="transform: translateX({onForm ? '0' : '100%'})"
			aria-hidden={!onForm}
		>
			{#if page.kind === 'form'}
				{#key `${page.existing?.name ?? ''}:${page.initialName}`}
					<div class="p-3">
						<LabelForm
							{workspace}
							existing={page.existing}
							initialName={page.initialName}
							autofocus={page.existing == undefined}
							onBack={backToList}
							{onSaved}
						/>
					</div>
				{/key}
			{/if}
		</div>
	</div>
</GenericDropdown>
