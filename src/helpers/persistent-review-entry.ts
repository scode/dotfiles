/**
 * The builder copies this template beside the helper snapshot and substitutes its source digest.
 * Deno compilation type-checks both files; this template is not an independently runnable script.
 */
import { pathToFileURL } from "node:url";

const expected = "__SOURCE_SHA256__";
try {
  // Published executables live at <skill>/bin/<platform>/evidence. Never resolve shipped source from the build path.
  const source = new URL("../../evidence.ts", pathToFileURL(Deno.execPath()));
  const hash = await crypto.subtle.digest("SHA-256", await Deno.readFile(source));
  const actual = Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, "0")).join("");
  if (actual !== expected) throw new Error("evidence.ts has changed");
} catch (error) {
  console.error(
    `PERSISTENT_REVIEW_TOOL_FAILED: compiled evidence helper is stale or its source is unavailable: ${error}. Rebuild helpers before syncing this skill. No command was executed.`,
  );
  Deno.exit(2);
}
const { runCli } = await import("./evidence.ts");
await runCli();
