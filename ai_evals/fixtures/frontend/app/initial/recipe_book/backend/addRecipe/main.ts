import * as wmill from 'windmill-client'

interface Recipe {
	id: number
	name: string
	ingredients: string
	instructions: string
	created_at: string
}

export async function main({
	name,
	ingredients,
	instructions
}: {
	name: string
	ingredients: string
	instructions: string
}): Promise<Recipe> {
	const sql = wmill.datatable()
	return await sql`
		INSERT INTO public.recipes (name, ingredients, instructions)
		VALUES (${name}, ${ingredients}, ${instructions})
		RETURNING id, name, ingredients, instructions, created_at
	`.fetchOne()
}
