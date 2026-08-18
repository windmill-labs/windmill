import React from 'react'

interface EmptyStateProps {
	title: string
	description?: string
	icon?: string
	action?: React.ReactNode
}

export const EmptyState: React.FC<EmptyStateProps> = ({
	title,
	description,
	icon = '📊',
	action
}) => {
	return (
		<div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-gray-300 bg-white py-12 text-center">
			<div className="text-3xl" aria-hidden>
				{icon}
			</div>
			<h3 className="mt-3 text-sm font-semibold text-gray-700">{title}</h3>
			{description ? (
				<p className="mt-1 max-w-sm text-sm text-gray-500">{description}</p>
			) : null}
			{action ? <div className="mt-4">{action}</div> : null}
		</div>
	)
}
