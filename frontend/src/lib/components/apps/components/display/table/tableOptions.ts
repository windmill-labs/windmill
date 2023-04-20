import { getCoreRowModel, getPaginationRowModel, type TableOptions } from '@tanstack/svelte-table'

const tableOptions: TableOptions<Record<string, any>> = {
	data: [],
	columns: [],
	enableColumnResizing: false,
	getCoreRowModel: getCoreRowModel(),
	getPaginationRowModel: getPaginationRowModel(),
	initialState: {
		pagination: {
			pageSize: 25
		}
	}
}

export { tableOptions }
