import * as restate from "@restatedev/restate-sdk";
import { benchmarkWorkflow } from "./workflow";

restate.endpoint().bind(benchmarkWorkflow).listen(9080);
console.log("Restate app server listening on port 9080");
