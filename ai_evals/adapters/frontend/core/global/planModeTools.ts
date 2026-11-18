import {
  EXIT_PLAN_MODE_TOOL,
  EXIT_PLAN_MODE_TOOL_DESCRIPTION,
  derivePlanTitle,
  exitPlanModeArgs,
  planSummaryOf,
} from "../../../../../frontend/src/lib/components/copilot/chat/planMode";
import { PLAN_MODE_MESSAGES } from "../../../../../frontend/src/lib/components/copilot/chat/planModeMessages";
import { createToolDef } from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import type { Tool as ProductionTool } from "../../../../../frontend/src/lib/components/copilot/chat/shared";

/**
 * `exit_plan_mode` built from the production schema, description and messages, so a case
 * exercises the real gate and wording with the posture living here rather than on the
 * manager. It resolves immediately — the runners define no `requestConfirmation`, so the
 * plan is always approved and a refused one cannot be expressed.
 */
export function createEvalPlanTools(artifacts: {
  create: (
    sessionId: string,
    input: Record<string, unknown>,
  ) => Promise<{ id: string; name: string }>;
  sessionId: string;
  chatId: string;
}): {
  tools: ProductionTool<{}>[];
  isPlanModeActive: () => boolean;
  isToolAvailable: (name: string) => boolean;
} {
  let planActive = true;
  return {
    isPlanModeActive: () => planActive,
    // Withdrawn on approval, as production's tool getter does it: leaving it advertised
    // invites a second hand-over of a plan already agreed, which would write a duplicate.
    // Production would offer enter_plan_mode in its place; these cases stop at the first
    // hand-over, so a fresh planning round belongs to a case of its own.
    isToolAvailable: (name) => name !== EXIT_PLAN_MODE_TOOL || planActive,
    // Production offers one plan tool at a time and these cases start in plan mode, so
    // enter_plan_mode would only invite a turn spent entering a posture already held.
    tools: [
      {
        def: createToolDef(
          exitPlanModeArgs,
          EXIT_PLAN_MODE_TOOL,
          EXIT_PLAN_MODE_TOOL_DESCRIPTION,
        ),
        // Carries the safety tag for the same reason production does: it is the only way out
        // of the posture, so the gate must not refuse it.
        planModeSafe: true,
        fn: async ({ args }) => {
          const summary = planSummaryOf(args);
          if (!summary?.trim()) {
            return PLAN_MODE_MESSAGES.missingSummary;
          }
          planActive = false;
          await artifacts.create(artifacts.sessionId, {
            name: derivePlanTitle(summary),
            content: summary,
            kind: "md",
            role: "plan",
            approvedVersion: 1,
            chatId: artifacts.chatId,
          });
          return PLAN_MODE_MESSAGES.approvedWithDoc;
        },
      },
    ] as ProductionTool<{}>[],
  };
}
