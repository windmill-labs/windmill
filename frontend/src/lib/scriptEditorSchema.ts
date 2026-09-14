import type { Schema, SupportedLanguage } from '$lib/common'
import type { Script } from '$lib/gen'
import { inferArgs } from '$lib/infer'
import { emptySchema } from '$lib/utils'
import { parsePipelineAnnotations } from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'

/**
 * A `// partitioned` pipeline script is materialized one slice at a time and
 * receives the slice as a runtime `partition` arg (the cascade injects it in
 * production). It isn't a code parameter, so schema inference doesn't see it —
 * surface it in the test form so a partitioned script can be run manually.
 */
export function injectPartitionArg(
	s: any,
	a: Record<string, any> | undefined,
	l: string | undefined,
	c: string
) {
	try {
		if (l !== 'duckdb' || !s?.properties) return
		const part = parsePipelineAnnotations(c).partition
		if (!part) return
		// Date-based partition kinds render a date / datetime picker; a dynamic
		// key is a free-form string.
		const format =
			part.kind === 'hourly'
				? 'date-time'
				: part.kind === 'daily' || part.kind === 'weekly' || part.kind === 'monthly'
					? 'date'
					: undefined
		if (!s.properties['partition']) {
			s.properties['partition'] = {
				type: 'string',
				...(format ? { format } : {}),
				// ISO output so partition keys sort lexicographically (the date
				// picker defaults to dd-MM-yyyy otherwise).
				...(format === 'date' ? { dateFormat: 'yyyy-MM-dd' } : {}),
				description:
					part.kind === 'dynamic'
						? 'Partition key value to materialize.'
						: `Partition (${part.kind}) to materialize.`
			}
			if (Array.isArray(s.order) && !s.order.includes('partition')) {
				s.order = ['partition', ...s.order]
			}
		}
		// Pre-fill the *test* arg with the current slice for date kinds — a
		// convenience default, kept on the args (not baked into the schema,
		// where it would persist to the deployed script and go stale).
		if (format && a && (a['partition'] == null || a['partition'] === '')) {
			const now = new Date()
			a['partition'] =
				format === 'date' ? now.toISOString().slice(0, 10) : now.toISOString().slice(0, 16)
		}
	} catch (e) {}
}

/**
 * The schema the script editor holds right after mounting on `content`, without
 * any user input: `ScriptEditor.inferSchema` re-infers the main signature into
 * the stored schema and injects the duckdb partition arg. A schema stored by
 * another producer (a CLI push, an older parser) can lack keys the current
 * parser emits, so the editor rewrites it on mount; a draft baseline taken from
 * the raw stored schema would then never compare equal to the untouched editor.
 * Returns a copy; a parse failure leaves it as far as inference got, which is
 * also what the editor keeps.
 */
export async function schemaAsEditorMounts(
	language: SupportedLanguage | undefined,
	content: string,
	schema: unknown,
	kind: Script['kind'] | undefined
): Promise<Schema> {
	const copy: Schema = schema ? structuredClone(schema as Schema) : emptySchema()
	const infer = async () => {
		await inferArgs(language, content, copy, kind === 'preprocessor' ? 'preprocessor' : undefined)
		injectPartitionArg(copy, undefined, language, content)
	}
	try {
		await infer()
	} catch {
		// The editors retry a failed mount inference once (transient wasm init);
		// the baseline has to land where that retry leaves the editor.
		if (!content || !language) return copy
		try {
			await infer()
		} catch {}
	}
	return copy
}
