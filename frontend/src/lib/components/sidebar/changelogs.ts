export type Changelog = {
	label: string
	href: string
	date: string
}

const changelogs: Changelog[] = [
	{
		label: 'Ag Charts',
		href: 'https://www.windmill.dev/changelog/ag-charts',
		date: '2024-01-24'
	},
	{
		label: 'Database Studio',
		href: 'https://www.windmill.dev/changelog/database-studio',
		date: '2024-01-24'
	},
	{
		label: 'Rich results render',
		href: 'https://www.windmill.dev/changelog/rich-render',
		date: '2024-01-23'
	}
]

export { changelogs }
