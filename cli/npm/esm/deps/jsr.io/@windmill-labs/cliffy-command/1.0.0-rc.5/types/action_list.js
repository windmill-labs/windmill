import { StringType } from "./string.js";
/** Completion list type. */
export class ActionListType extends StringType {
    constructor(cmd) {
        super();
        Object.defineProperty(this, "cmd", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: cmd
        });
    }
    /** Complete action names. */
    complete() {
        return this.cmd.getCompletions()
            .map((type) => type.name)
            // filter unique values
            .filter((value, index, self) => self.indexOf(value) === index);
    }
}
