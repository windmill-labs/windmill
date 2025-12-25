import React from 'react'

interface BreadcrumbProps {
	path: { id: string | null; name: string }[]
	onNavigate: (folderId: string | null, folderName: string) => void
}

export const Breadcrumb: React.FC<BreadcrumbProps> = ({ path, onNavigate }) => {
	return (
		<div className="px-4 py-2 bg-white border-b flex items-center gap-2 text-sm">
			{path.map((item, index) => (
				<React.Fragment key={item.id ?? 'root'}>
					{index > 0 && <span className="text-gray-400">/</span>}
					<button
						onClick={() => onNavigate(item.id, item.name)}
						className={`hover:text-blue-600 ${
							index === path.length - 1 ? 'font-medium text-gray-900' : 'text-gray-600'
						}`}
					>
						{item.name}
					</button>
				</React.Fragment>
			))}
		</div>
	)
}
