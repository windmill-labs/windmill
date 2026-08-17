import React, { useEffect, useState } from 'react'
import { backend } from 'wmill'

interface Recipe {
	id: number
	name: string
	ingredients: string
	instructions: string
	created_at: string
}

const emptyForm = {
	name: '',
	ingredients: '',
	instructions: ''
}

const RecipeBookApp = () => {
	const [recipes, setRecipes] = useState<Recipe[]>([])
	const [form, setForm] = useState(emptyForm)
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		void loadRecipes()
	}, [])

	const loadRecipes = async () => {
		try {
			setLoading(true)
			setError(null)
			const data = await backend.listRecipes()
			setRecipes(data)
		} catch (error) {
			console.error(error)
			setError('Failed to load recipes')
		} finally {
			setLoading(false)
		}
	}

	const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		const name = form.name.trim()
		const ingredients = form.ingredients.trim()
		const instructions = form.instructions.trim()

		if (!name || !ingredients || !instructions) {
			setError('Please fill in every field')
			return
		}

		try {
			setError(null)
			const recipe = await backend.addRecipe({
				name,
				ingredients,
				instructions
			})
			setRecipes((previous) => [recipe, ...previous])
			setForm(emptyForm)
		} catch (error) {
			console.error(error)
			setError('Failed to save recipe')
		}
	}

	return (
		<div className="min-h-screen bg-stone-100 p-6">
			<div className="mx-auto max-w-5xl rounded-3xl bg-white shadow-lg shadow-stone-300/40">
				<div className="border-b border-stone-200 px-8 py-6">
					<h1 className="text-3xl font-semibold text-stone-900">Recipe book</h1>
					<p className="mt-2 text-sm text-stone-600">
						Add recipes and keep them stored in the existing datatable-backed app.
					</p>
				</div>

				<div className="grid gap-8 px-8 py-8 lg:grid-cols-[320px_1fr]">
					<form className="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-5" onSubmit={handleSubmit}>
						<h2 className="text-lg font-medium text-stone-900">Add recipe</h2>
						<label className="block space-y-2">
							<span className="text-sm font-medium text-stone-700">Name</span>
							<input
								className="w-full rounded-xl border border-stone-300 bg-white px-3 py-2 text-sm"
								value={form.name}
								onChange={(event) => setForm({ ...form, name: event.target.value })}
							/>
						</label>
						<label className="block space-y-2">
							<span className="text-sm font-medium text-stone-700">Ingredients</span>
							<textarea
								className="min-h-28 w-full rounded-xl border border-stone-300 bg-white px-3 py-2 text-sm"
								value={form.ingredients}
								onChange={(event) => setForm({ ...form, ingredients: event.target.value })}
							/>
						</label>
						<label className="block space-y-2">
							<span className="text-sm font-medium text-stone-700">Instructions</span>
							<textarea
								className="min-h-32 w-full rounded-xl border border-stone-300 bg-white px-3 py-2 text-sm"
								value={form.instructions}
								onChange={(event) => setForm({ ...form, instructions: event.target.value })}
							/>
						</label>
						<button
							type="submit"
							className="rounded-full bg-stone-900 px-4 py-2 text-sm font-medium text-white"
						>
							Save recipe
						</button>
					</form>

					<div className="space-y-4">
						{error ? (
							<div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
								{error}
							</div>
						) : null}

						{loading ? (
							<div className="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-10 text-center text-sm text-stone-500">
								Loading recipes...
							</div>
						) : recipes.length === 0 ? (
							<div className="rounded-2xl border border-dashed border-stone-300 px-4 py-10 text-center text-sm text-stone-500">
								No recipes saved yet.
							</div>
						) : (
							<ul className="space-y-4">
								{recipes.map((recipe) => (
									<li key={recipe.id} className="rounded-2xl border border-stone-200 p-5">
										<div className="flex items-start justify-between gap-4">
											<div>
												<h3 className="text-lg font-medium text-stone-900">{recipe.name}</h3>
												<p className="mt-1 text-xs uppercase tracking-[0.18em] text-stone-500">
													Stored recipe
												</p>
											</div>
											<span className="rounded-full bg-stone-100 px-3 py-1 text-xs text-stone-600">
												{new Date(recipe.created_at).toLocaleDateString()}
											</span>
										</div>
										<div className="mt-4 grid gap-4 md:grid-cols-2">
											<div>
												<h4 className="text-sm font-medium text-stone-700">Ingredients</h4>
												<p className="mt-2 whitespace-pre-wrap text-sm text-stone-600">
													{recipe.ingredients}
												</p>
											</div>
											<div>
												<h4 className="text-sm font-medium text-stone-700">Instructions</h4>
												<p className="mt-2 whitespace-pre-wrap text-sm text-stone-600">
													{recipe.instructions}
												</p>
											</div>
										</div>
									</li>
								))}
							</ul>
						)}
					</div>
				</div>
			</div>
		</div>
	)
}

export default RecipeBookApp
