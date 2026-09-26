/** `result` of a draft write that did not land. The write reports it through `result` alone, not
 * `error`, so a tool group reads these to count the call as failed. */
export const DRAFT_CONFLICT_RESULT = 'Conflict'
export const DRAFT_SAVE_FAILED_RESULT = 'Save failed'
