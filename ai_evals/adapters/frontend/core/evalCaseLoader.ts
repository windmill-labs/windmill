import type { ScriptLang } from "$lib/gen/types.gen";
import {
  loadEvalCases,
  type EvalScriptFixture
} from "../../shared/evalCases";

export interface FlowEvalCaseManifest {
  id: string;
  title: string;
  userPrompt: string;
  minJudgeScore?: number;
}

export interface FlowEvalCase extends FlowEvalCaseManifest {
  expectedFlow: Record<string, unknown>;
  initialFlow?: Record<string, any>;
}

export interface AppEvalCaseManifest {
  id: string;
  title: string;
  userPrompt: string;
  initialAppFixturePath?: string;
  minJudgeScore?: number;
}

export interface AppEvalCase extends AppEvalCaseManifest {}

export interface ScriptEvalFixture {
  code: string;
  lang: ScriptLang | "bunnative";
  path: string;
  args?: Record<string, any>;
}

export interface ScriptEvalCaseManifest {
  id: string;
  title: string;
  userPrompt: string;
  minJudgeScore?: number;
}

export interface ScriptEvalCase extends ScriptEvalCaseManifest {
  expectedScript: ScriptEvalFixture;
  initialScript?: ScriptEvalFixture;
}

export function loadFlowEvalCases(): FlowEvalCase[] {
  return loadEvalCases("frontend-flow").map((testCase) => ({
    id: testCase.id,
    title: testCase.title,
    userPrompt: testCase.userPrompt,
    minJudgeScore: testCase.judgeRubric.minScore,
    expectedFlow: testCase.artifactChecks.expectedFlow,
    initialFlow: testCase.initialState.initialFlow as Record<string, any> | undefined
  }));
}

export function loadAppEvalCases(): AppEvalCase[] {
  return loadEvalCases("frontend-app").map((testCase) => ({
    id: testCase.id,
    title: testCase.title,
    userPrompt: testCase.userPrompt,
    initialAppFixturePath: testCase.initialState.initialAppFixturePath,
    minJudgeScore: testCase.judgeRubric.minScore
  }));
}

export function loadScriptEvalCases(): ScriptEvalCase[] {
  return loadEvalCases("frontend-script").map((testCase) => ({
    id: testCase.id,
    title: testCase.title,
    userPrompt: testCase.userPrompt,
    minJudgeScore: testCase.judgeRubric.minScore,
    expectedScript: toScriptEvalFixture(testCase.artifactChecks.expectedScript),
    initialScript: testCase.initialState.initialScript
      ? toScriptEvalFixture(testCase.initialState.initialScript)
      : undefined
  }));
}

function toScriptEvalFixture(fixture: EvalScriptFixture): ScriptEvalFixture {
  return {
    code: fixture.code,
    lang: fixture.lang as ScriptLang | "bunnative",
    path: fixture.path,
    args: fixture.args as Record<string, any> | undefined
  };
}
