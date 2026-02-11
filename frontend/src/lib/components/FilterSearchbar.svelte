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
	export function buildFilterInstance<T extends FilterSchemaRec>(def: T): FilterInstanceRec<T> {
		let r = {} as FilterInstanceRec<T>
		for (const key in def) {
			const filterSchema = def[key]
			if (filterSchema.type === 'string') {
				r[key] = { type: 'string', value: '' } as any
			} else if (filterSchema.type === 'number') {
				r[key] = { type: 'number', value: 0 } as any
			} else if (filterSchema.type === 'boolean') {
				r[key] = { type: 'boolean', value: false } as any
			} else if (filterSchema.type === 'date') {
				r[key] = { type: 'date', value: new Date() } as any
			} else if (filterSchema.type === 'oneof') {
				r[key] = { type: 'oneof', value: filterSchema.options[0] } as any
			}
		}
		return r
	}
</script>

<script lang="ts">
	type Props = {
		schema: FilterSchemaRec
		value: FilterInstanceRec<FilterSchemaRec>
	}

	let { schema, value }: Props = $props()
</script>
