import React from 'react'
import type { OrderStatus } from '../data/seedData'
import { STATUS_LABELS } from '../data/seedData'

interface StatusBadgeProps {
	status: OrderStatus
}

const STATUS_STYLES: Record<OrderStatus, string> = {
	paid: 'bg-blue-100 text-blue-700',
	shipped: 'bg-indigo-100 text-indigo-700',
	delivered: 'bg-emerald-100 text-emerald-700',
	pending: 'bg-amber-100 text-amber-700',
	refunded: 'bg-rose-100 text-rose-700',
	cancelled: 'bg-gray-200 text-gray-600'
}

export const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
	return (
		<span
			className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${STATUS_STYLES[status]}`}
		>
			{STATUS_LABELS[status]}
		</span>
	)
}
