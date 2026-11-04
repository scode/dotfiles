/**
 * The builder copies this template beside the helper snapshot and substitutes its source digest.
 * Deno compilation type-checks both files; this template is not an independently runnable script.
 */
import { pathToFileURL } from "node:url";

const expected = "__SOURCE_SHA256__";
try {
  // Published executables live at <skill>/bin/<platform>/schedule. Never resolve shipped source from the build path.
  const source = new URL("../../schedule.ts", pathToFileURL(Deno.execPath()));
  const hash = await crypto.subtle.digest("SHA-256", await Deno.readFile(source));
  const actual = Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, "0")).join("");
  if (actual !== expected) throw new Error("schedule.ts has changed");
} catch (error) {
  console.error(
    `PERSISTENT_REVIEW_SCHEDULE_FAILED: compiled schedule helper is stale or its source is unavailable: ${error}. Rebuild helpers before syncing this skill. No command was executed.`,
  );
  Deno.exit(2);
}
const { runCli } = await import("./schedule.ts");
await runCli();
