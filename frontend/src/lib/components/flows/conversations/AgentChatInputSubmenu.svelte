<script lang="ts">
	import { untrack } from 'svelte'
	import { melt } from '@melt-ui/svelte'
	import { twMerge } from 'tailwind-merge'
	import { ChevronRight } from 'lucide-svelte'
	import type { MenubarMenuElements, createDropdownMenu } from '@melt-ui/svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { deepEqual } from 'fast-equals'
	import { type DynamicInput } from '$lib/utils'
	import { AGENT_CHAT_INPUT_META, type AgentChatInput } from './agentChatInputs'

	interface Props {
		input: AgentChatInput
		value: any
		onChange: (value: any) => void
		builders: ReturnType<typeof createDropdownMenu>['builders']
		meltItem: MenubarMenuElements['item']
		workspace?: string
		helperScript?: DynamicInput.HelperScript
	}

	let { input, value, onChange, builders, workspace, helperScript }: Props = $props()

	const {
		elements: { subTrigger, subMenu },
		states: { subOpen }
	} = untrack(() => builders).createSubmenu()

	const meta = $derived(AGENT_CHAT_INPUT_META[input.key])
	const summary = $derived(meta.summarize(value))

	// A one-property schema, so the submenu holds the exact editor the Configure-inputs
	// modal would render for this input.
	const fieldSchema = $derived({
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		type: 'object',
		properties: { [input.name]: input.property },
		required: input.required ? [input.name] : [],
		order: [input.name]
	})

	// SchemaForm writes into `args` in place, so it cannot drive a function binding.
	// Both directions are kept in step: out on an edit, and back in when the owner
	// resets the value from outside.
	let synced = $state.snapshot(value)
	let args = $state<Record<string, any>>({ [input.name]: synced })

	$effect(() => {
		const edited = $state.snapshot(args[input.name])
		if (!deepEqual(edited, synced)) {
			synced = edited
			onChange(edited)
		}
	})

	$effect(() => {
		const incoming = $state.snapshot(value)
		if (
			!deepEqual(
				incoming,
				untrack(() => synced)
			)
		) {
			synced = incoming
			args = { [input.name]: incoming }
		}
	})

	// Melt's roving focus blurs the focused element on pointermove, which would abort a
	// native drag or steal focus mid-typing. Direct listeners so they run before melt's.
	function isolatePointer(node: HTMLElement) {
		const stop = (e: Event) => e.stopPropagation()
		node.addEventListener('pointerdown', stop)
		node.addEventListener('pointermove', stop)
		node.addEventListener('keydown', stop)
		return {
			destroy() {
				node.removeEventListener('pointerdown', stop)
				node.removeEventListener('pointermove', stop)
				node.removeEventListener('keydown', stop)
			}
		}
	}
</script>

<button
	use:melt={$subTrigger}
	class={twMerge(
		'px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full',
		'data-[highlighted]:bg-surface-hover',
		'flex flex-row gap-2 items-center rounded-sm'
	)}
>
	<meta.icon size={14} class="shrink-0" />
	<p class="truncate grow min-w-0 whitespace-nowrap text-left">{meta.label}</p>
	{#if summary}
		<span class="shrink-0 text-tertiary truncate max-w-[80px]">{summary}</span>
	{/if}
	<ChevronRight size={14} class="ml-auto shrink-0 text-tertiary" />
</button>

{#if $subOpen}
	<div
		use:melt={$subMenu}
		class="z-[6000] bg-surface-tertiary dark:border w-72 origin-top-right rounded-lg shadow-lg focus:outline-none p-3"
	>
		<div use:isolatePointer>
			<SchemaForm schema={fieldSchema} bind:args {helperScript} {workspace} />
		</div>
	</div>
{/if}
