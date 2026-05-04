import React, { useEffect, useState } from 'react'
import { backend } from 'wmill'

interface InventoryItem {
	id: number
	name: string
	sku: string
	quantity: number
	price: number
	created_at: string
}

const emptyForm = {
	name: '',
	sku: '',
	quantity: '1',
	price: '0.00'
}

const InventoryTrackerApp = () => {
	const [items, setItems] = useState<InventoryItem[]>([])
	const [form, setForm] = useState(emptyForm)
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		void loadItems()
	}, [])

	const loadItems = async () => {
		try {
			setLoading(true)
			setError(null)
			const data = await backend.listInventory()
			setItems(data)
		} catch (error) {
			console.error(error)
			setError('Failed to load inventory')
		} finally {
			setLoading(false)
		}
	}

	const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		const name = form.name.trim()
		const sku = form.sku.trim()
		const quantity = Number(form.quantity)
		const price = Number(form.price)

		if (!name || !sku || Number.isNaN(quantity) || Number.isNaN(price) || quantity < 0 || price < 0) {
			setError('Enter a valid name, sku, quantity, and price')
			return
		}

		try {
			setError(null)
			const item = await backend.addInventory({
				name,
				sku,
				quantity,
				price
			})
			setItems((previous) => [item, ...previous])
			setForm(emptyForm)
		} catch (error) {
			console.error(error)
			setError('Failed to save inventory item')
		}
	}

	return (
		<div className="min-h-screen bg-slate-100 p-6">
			<div className="mx-auto max-w-6xl rounded-3xl bg-white shadow-lg shadow-slate-300/40">
				<div className="border-b border-slate-200 px-8 py-6">
					<h1 className="text-3xl font-semibold text-slate-900">Inventory tracker</h1>
					<p className="mt-2 text-sm text-slate-600">
						Add products and keep them stored in the existing datatable-backed app.
					</p>
				</div>

				<div className="grid gap-8 px-8 py-8 lg:grid-cols-[340px_1fr]">
					<form className="space-y-4 rounded-2xl border border-slate-200 bg-slate-50 p-5" onSubmit={handleSubmit}>
						<h2 className="text-lg font-medium text-slate-900">Add item</h2>
						<label className="block space-y-2">
							<span className="text-sm font-medium text-slate-700">Name</span>
							<input
								className="w-full rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm"
								value={form.name}
								onChange={(event) => setForm({ ...form, name: event.target.value })}
							/>
						</label>
						<label className="block space-y-2">
							<span className="text-sm font-medium text-slate-700">SKU</span>
							<input
								className="w-full rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm"
								value={form.sku}
								onChange={(event) => setForm({ ...form, sku: event.target.value })}
							/>
						</label>
						<div className="grid gap-4 sm:grid-cols-2">
							<label className="block space-y-2">
								<span className="text-sm font-medium text-slate-700">Quantity</span>
								<input
									type="number"
									min="0"
									className="w-full rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm"
									value={form.quantity}
									onChange={(event) => setForm({ ...form, quantity: event.target.value })}
								/>
							</label>
							<label className="block space-y-2">
								<span className="text-sm font-medium text-slate-700">Price</span>
								<input
									type="number"
									min="0"
									step="0.01"
									className="w-full rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm"
									value={form.price}
									onChange={(event) => setForm({ ...form, price: event.target.value })}
								/>
							</label>
						</div>
						<button
							type="submit"
							className="rounded-full bg-slate-900 px-4 py-2 text-sm font-medium text-white"
						>
							Save item
						</button>
					</form>

					<div className="space-y-4">
						{error ? (
							<div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
								{error}
							</div>
						) : null}

						{loading ? (
							<div className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-10 text-center text-sm text-slate-500">
								Loading inventory...
							</div>
						) : items.length === 0 ? (
							<div className="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500">
								No items saved yet.
							</div>
						) : (
							<div className="overflow-hidden rounded-2xl border border-slate-200">
								<table className="min-w-full divide-y divide-slate-200 text-sm">
									<thead className="bg-slate-50 text-left text-slate-600">
										<tr>
											<th className="px-4 py-3 font-medium">Item</th>
											<th className="px-4 py-3 font-medium">SKU</th>
											<th className="px-4 py-3 font-medium">Qty</th>
											<th className="px-4 py-3 font-medium">Price</th>
										</tr>
									</thead>
									<tbody className="divide-y divide-slate-200 bg-white">
										{items.map((item) => (
											<tr key={item.id}>
												<td className="px-4 py-3 text-slate-900">
													<div className="font-medium">{item.name}</div>
													<div className="text-xs text-slate-500">
														Added {new Date(item.created_at).toLocaleDateString()}
													</div>
												</td>
												<td className="px-4 py-3 text-slate-600">{item.sku}</td>
												<td className="px-4 py-3 text-slate-600">{item.quantity}</td>
												<td className="px-4 py-3 text-slate-600">
													${item.price.toFixed(2)}
												</td>
											</tr>
										))}
									</tbody>
								</table>
							</div>
						)}
					</div>
				</div>
			</div>
		</div>
	)
}

export default InventoryTrackerApp
