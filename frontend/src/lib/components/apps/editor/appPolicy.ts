import { getCountInput } from '../components/display/dbtable/queries/count'
import { getDeleteInput } from '../components/display/dbtable/queries/delete'
import { getInsertInput } from '../components/display/dbtable/queries/insert'
import { getSelectInput } from '../components/display/dbtable/queries/select'
import { getUpdateInput } from '../components/display/dbtable/queries/update'
import { getPrimaryKeys, type ColumnDef } from '../components/display/dbtable/utils'
import { isRunnableByName, isRunnableByPath, type AppInput, type Runnable } from '../inputType'
import type { App } from '../types'
import {
	computeS3FileInputPolicy,
	computeWorkspaceS3FileInputPolicy,
	computeS3FileViewerPolicy
} from './appUtilsS3'
import { collectStaticFields, type TriggerableV2 } from './commonAppUtils'
import type { Policy } from '$lib/gen'
import type { AppComponent } from './component/components'
import { collectOneOfFields, allItems, BG_PREFIX } from './appUtilsCore'
import type { DbInput, DbType } from '$lib/components/dbTypes'

export async function updatePolicy(app: App, currentPolicy: Policy | undefined): Promise<Policy> {
	const items = allItems(app.grid, app.subgrids)
	const allTriggers: ([string, TriggerableV2] | undefined)[] = (await Promise.all(
		items
			.flatMap((x) => {
				let c = x.data as AppComponent
				let r: { input: AppInput | undefined; id: string }[] = [
					{ input: c.componentInput, id: x.id }
				]
				if (c.type === 'tablecomponent') {
					r.push(...c.actionButtons.map((x) => ({ input: x.componentInput, id: x.id })))
				}
				if (
					(c.type === 'aggridcomponent' ||
						c.type === 'aggridcomponentee' ||
						c.type === 'dbexplorercomponent' ||
						c.type === 'aggridinfinitecomponent' ||
						c.type === 'aggridinfinitecomponentee') &&
					Array.isArray(c.actions)
				) {
					r.push(...c.actions.map((x) => ({ input: x.componentInput, id: x.id })))
				}
				if (c.type === 'menucomponent') {
					r.push(...c.menuItems.map((x) => ({ input: x.componentInput, id: x.id })))
				}
				if (c.type === 'dbexplorercomponent') {
					let nr: { id: string; input: AppInput }[] = []
					let config = c.configuration as any

					const dbType = (
						config?.type?.selected === 'datatable' ? 'postgresql' : config?.type?.selected
					) as DbType
					let pg = config?.type?.configuration?.[config?.type?.selected]

					if (pg && dbType) {
						const { table, resource, ducklake, datatable } = pg
						const tableValue = table.value
						const dbPath =
							resource?.value.split('$res:')[1] ??
							(ducklake?.value as string | undefined)?.split('ducklake://')[1] ??
							datatable?.value
						const columnDefs = (c.configuration.columnDefs as any).value as ColumnDef[]
						const whereClause = (c.configuration.whereClause as any).value as unknown as
							| string
							| undefined
						if (tableValue && dbPath && columnDefs) {
							let dbInput: DbInput = ducklake
								? { type: 'ducklake', ducklake: dbPath }
								: { type: 'database', resourcePath: dbPath, resourceType: dbType }
							r.push({
								input: getSelectInput(dbInput, tableValue, columnDefs, whereClause),
								id: x.id
							})

							r.push({
								input: getCountInput(dbInput, tableValue, columnDefs, whereClause),
								id: x.id + '_count'
							})

							r.push({
								input: getInsertInput(dbInput, tableValue, columnDefs),
								id: x.id + '_insert'
							})

							let primaryColumns = getPrimaryKeys(columnDefs)
							let columns = columnDefs?.filter((x) => primaryColumns.includes(x.field))

							r.push({
								input: getDeleteInput(dbInput, tableValue, columns),
								id: x.id + '_delete'
							})

							columnDefs
								.filter((col) => col.editable || config.allEditable.value)
								.forEach((column) => {
									r.push({
										input: getUpdateInput(dbInput, tableValue, column, columns),
										id: x.id + '_update'
									})
								})
						}
					}
					r.push(...nr)
				}

				const processed = r
					.filter((x) => x.input)
					.map(async (o) => {
						if (o.input?.type == 'runnable') {
							return await processRunnable(o.id, o.input.runnable, o.input.fields, app)
						}
					})

				return processed as Promise<[string, TriggerableV2] | undefined>[]
			})
			.concat(
				Object.values(app.hiddenInlineScripts ?? {}).map(async (v, i) => {
					return await processRunnable(BG_PREFIX + i, v, v.fields, app)
				}) as Promise<[string, TriggerableV2] | undefined>[]
			)
	)) as ([string, TriggerableV2] | undefined)[]

	const ntriggerables: Record<string, TriggerableV2> = Object.fromEntries(
		allTriggers.filter(Boolean) as [string, TriggerableV2][]
	)

	const s3_inputs = items
		.filter((x) => (x.data as AppComponent).type === 's3fileinputcomponent')
		.map((x) => {
			const c = x.data as AppComponent
			const config = c.configuration as any
			return computeS3FileInputPolicy(config?.type?.configuration?.s3, app)
		})
		.filter(Boolean) as {
		allowed_resources: string[]
		allow_user_resources: boolean
		file_key_regex: string
	}[]

	if (
		items.findIndex((x) => {
			const c = x.data as AppComponent
			if (
				c.type === 'schemaformcomponent' ||
				c.type === 'formbuttoncomponent' ||
				c.type === 'formcomponent'
			) {
				const props =
					c.type === 'schemaformcomponent'
						? (c.componentInput as any)?.value?.properties
						: isRunnableByName((c.componentInput as any)?.runnable)
							? (c.componentInput as any)?.runnable?.inlineScript?.schema?.properties
							: (c.componentInput as any)?.runnable?.schema?.properties
				return (
					Object.values(props ?? {}).findIndex(
						(p: any) =>
							(p?.type === 'object' && p?.format === 'resource-s3_object') ||
							(p?.type === 'array' &&
								(p?.items?.resourceType === 's3object' || p?.items?.resourceType === 's3_object'))
					) !== -1
				)
			} else {
				return false
			}
		}) !== -1
	) {
		s3_inputs.push(computeWorkspaceS3FileInputPolicy())
	}

	const s3FileKeys = items
		.filter(
			(x) =>
				(x.data as AppComponent).type === 'imagecomponent' ||
				(x.data as AppComponent).type === 'pdfcomponent' ||
				(x.data as AppComponent).type === 'downloadcomponent'
		)
		.map((x) => {
			const c = x.data as AppComponent
			const config = c.configuration
			return computeS3FileViewerPolicy(config)
		})
		.filter(Boolean) as { s3_path: string; storage?: string | undefined }[]

	return {
		...(currentPolicy ?? {}),
		allowed_s3_keys: s3FileKeys,
		s3_inputs,
		triggerables_v2: ntriggerables
	}
}

export async function processRunnable(
	id: string,
	runnable: Runnable,
	fields: Record<string, any>,
	app: App
): Promise<[string, TriggerableV2] | undefined> {
	const staticInputs = collectStaticFields(fields)
	const oneOfInputs = collectOneOfFields(fields, app)
	const allowUserResources: string[] = Object.entries(fields)
		.map(([k, v]) => {
			return v['allowUserResources'] ? k : undefined
		})
		.filter(Boolean) as string[]

	if (isRunnableByName(runnable)) {
		let hex = await hash(runnable.inlineScript?.content)
		console.debug('hex', hex, id)
		return [
			`${id}:rawscript/${hex}`,
			{
				static_inputs: staticInputs,
				one_of_inputs: oneOfInputs,
				allow_user_resources: allowUserResources
			}
		]
	} else if (isRunnableByPath(runnable)) {
		let prefix = runnable.runType !== 'hubscript' ? runnable.runType : 'script'
		return [
			`${id}:${prefix}/${runnable.path}`,
			{
				static_inputs: staticInputs,
				one_of_inputs: oneOfInputs,
				allow_user_resources: allowUserResources
			}
		]
	}
}

async function hash(message) {
	try {
		const msgUint8 = new TextEncoder().encode(message) // encode as (utf-8) Uint8Array
		const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8) // hash the message
		const hashArray = Array.from(new Uint8Array(hashBuffer)) // convert buffer to byte array
		const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
		return hashHex
	} catch {
		//subtle not available, trying pure js
		const { Sha256 } = await import('@aws-crypto/sha256-js')
		const hash = new Sha256()
		hash.update(message ?? '')
		const result = Array.from(await hash.digest())
		const hex = result.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
		return hex
	}
}
