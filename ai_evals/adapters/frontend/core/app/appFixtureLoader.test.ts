import { describe, expect, it } from "bun:test";
import { fileURLToPath } from "node:url";
import { loadAppFixture } from "./appFixtureLoader";

const RECIPE_BOOK_FIXTURE = fileURLToPath(
  new URL("../../../../fixtures/frontend/app/initial/recipe_book", import.meta.url)
);

describe("loadAppFixture", () => {
  it("loads datatables from app fixtures when present", async () => {
    const fixture = await loadAppFixture(RECIPE_BOOK_FIXTURE);

    expect(fixture.datatables).toEqual([
      {
        datatable_name: "main",
        schemas: {
          public: {
            recipes: {
              id: "int4",
              name: "text",
              ingredients: "text",
              instructions: "text",
              created_at: "timestamp=now()",
            },
          },
        },
      },
    ]);
  });
});
