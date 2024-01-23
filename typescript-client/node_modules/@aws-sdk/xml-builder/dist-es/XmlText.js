import { escapeElement } from "./escape-element";
export class XmlText {
    constructor(value) {
        this.value = value;
    }
    toString() {
        return escapeElement("" + this.value);
    }
}
