import { getCoreRowModel, getPaginationRowModel } from '@tanstack/svelte-table'

const tableOptions = {
	getCoreRowModel: getCoreRowModel(),
	getPaginationRowModel: getPaginationRowModel(),
	initialState: {
		pagination: {
			pageSize: 10
		}
	}
}

export { tableOptions }
