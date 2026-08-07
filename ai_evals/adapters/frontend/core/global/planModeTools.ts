import {
  EXIT_PLAN_MODE_TOOL,
  EXIT_PLAN_MODE_TOOL_DESCRIPTION,
  PLAN_MODE_MESSAGES,
  derivePlanTitle,
  exitPlanModeArgs,
} from "../../../../../frontend/src/lib/components/copilot/chat/planMode";
import { createToolDef } from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import type { Tool as ProductionTool } from "../../../../../frontend/src/lib/components/copilot/chat/shared";

/**
 * The two plan tools, assembled from the same schemas, descriptions and messages
 * production ships, over a plain flag instead of AIChatManager's autonomy state.
 *
 * The posture and the plan document are what an eval can see: the approved plan is saved
 * as an artifact, as production does, so the run's output carries the plan itself and not
 * just the fact that one was handed over. The rest of AIChatManager's lifecycle — the
 * rollback of a refused proposal, the generation guards — has no user to drive it here and
 * is covered by AIChatManager.test.ts.
 *
 * The eval runners define no `requestConfirmation`, so both tools resolve immediately,
 * standing in for a user who approves. A case needing a refused plan cannot be written.
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
} {
  let planActive = true;
  return {
    isPlanModeActive: () => planActive,
    // Production offers exactly one plan tool at a time, and these cases start in plan
    // mode, so enter_plan_mode is not among them — offered here it only invites the model
    // to spend a turn entering a posture it is already in.
    tools: [
      {
        def: createToolDef(
          exitPlanModeArgs,
          EXIT_PLAN_MODE_TOOL,
          EXIT_PLAN_MODE_TOOL_DESCRIPTION,
        ),
        // Carries `readonly` for the same reason production does: it is the only way out
        // of the posture, so the gate must not refuse it.
        readonly: true,
        fn: async ({ args }) => {
          const summary = exitPlanModeArgs.safeParse(args).data?.summary;
          if (!summary?.trim()) {
            return PLAN_MODE_MESSAGES.missingSummary;
          }
          planActive = false;
          await artifacts.create(artifacts.sessionId, {
            name: derivePlanTitle(summary),
            content: summary,
            kind: "md",
            role: "plan",
            approved: true,
            chatId: artifacts.chatId,
          });
          return PLAN_MODE_MESSAGES.approvedWithDoc;
        },
      },
    ] as ProductionTool<{}>[],
  };
}
