import React from 'react'

export type DashboardView = 'overview' | 'orders' | 'regions' | 'products'

interface SidebarProps {
	active: DashboardView
	onSelect: (view: DashboardView) => void
}

const NAV_ITEMS: { id: DashboardView; label: string; icon: string }[] = [
	{ id: 'overview', label: 'Overview', icon: '📈' },
	{ id: 'orders', label: 'Orders', icon: '🧾' },
	{ id: 'regions', label: 'Regions', icon: '🌍' },
	{ id: 'products', label: 'Products', icon: '📦' }
]

export const Sidebar: React.FC<SidebarProps> = ({ active, onSelect }) => {
	return (
		<aside className="flex w-56 flex-col border-r border-gray-200 bg-white">
			<div className="flex items-center gap-2 border-b border-gray-200 px-5 py-4">
				<span className="text-xl">🪁</span>
				<span className="text-sm font-bold text-gray-900">Acme Operations</span>
			</div>
			<nav className="flex-1 space-y-1 p-3">
				{NAV_ITEMS.map((item) => {
					const isActive = item.id === active
					return (
						<button
							key={item.id}
							type="button"
							onClick={() => onSelect(item.id)}
							className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm font-medium transition ${
								isActive
									? 'bg-indigo-50 text-indigo-700'
									: 'text-gray-600 hover:bg-gray-50'
							}`}
						>
							<span aria-hidden>{item.icon}</span>
							{item.label}
						</button>
					)
				})}
			</nav>
			<div className="border-t border-gray-200 p-4 text-xs text-gray-400">
				Analytics workspace
				<div className="mt-1 font-mono text-[10px] text-gray-300">v2.4.0</div>
			</div>
		</aside>
	)
}
