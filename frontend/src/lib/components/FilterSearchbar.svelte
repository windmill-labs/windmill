<script lang="ts" module>
	export type FilterSchemaRec = Record<string, FilterSchema>
	export type FilterSchema =
		| {
				type: 'string' | 'number' | 'boolean' | 'date'
		  }
		| {
				type: 'oneof'
				options: string[]
		  }
	export type FilterInstanceRec<T extends FilterSchemaRec> = {
		[K in keyof T]: FilterInstance<T[K]>
	}
	export type FilterInstance<T extends FilterSchema> = T extends { type: 'string' }
		? { type: 'string'; value: string }
		: T extends { type: 'number' }
			? { type: 'number'; value: number }
			: T extends { type: 'boolean' }
				? { type: 'boolean'; value: boolean }
				: T extends { type: 'date' }
					? { type: 'date'; value: Date }
					: T extends { type: 'oneof'; options: infer O }
						? { type: 'oneof'; value: O extends string[] ? O[number] : never }
						: never
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { inputBaseClass, inputBorderClass, inputSizeClasses } from './text_input/TextInput.svelte'
	import { SearchIcon } from 'lucide-svelte'

	type Props = {
		schema: FilterSchemaRec
		value: Partial<FilterInstanceRec<FilterSchemaRec>>
		class?: string
	}

	let { schema, value, class: className }: Props = $props()
</script>

<div
	contenteditable="true"
	class={twMerge(
		'bg-surface-input flex gap-2 items-center outline-none',
		inputBaseClass,
		inputBorderClass(),
		inputSizeClasses.md,
		className
	)}
>
	<SearchIcon size="16" class="ml-auto" />
</div>
