import * as wmill from 'windmill-client'

interface Recipe {
	id: number
	name: string
	ingredients: string
	instructions: string
	created_at: string
}

export async function main(): Promise<Recipe[]> {
	const sql = wmill.datatable()
	return await sql`
		SELECT id, name, ingredients, instructions, created_at
		FROM public.recipes
		ORDER BY created_at DESC
	`.fetch()
}
