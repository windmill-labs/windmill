/** What a reader set on an S3 table: kept by the result that shows it, so switching the
 * result to JSON and back, which remounts the table, keeps the separator, header row and
 * search. */
export class S3TableView {
	separator: string | undefined = $state(undefined)
	hasHeader = $state(true)
	filters: Record<string, any> = {}
}
