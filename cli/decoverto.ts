// globally shared decoverto instance
import { Decoverto } from "npm:decoverto";
const decoverto = new Decoverto();

// TODO: Properly type FlowModule

export { Any, array, map, MapShape, model, property } from "npm:decoverto";
export { decoverto };
