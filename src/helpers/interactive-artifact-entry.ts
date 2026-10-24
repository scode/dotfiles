/**
 * The builder copies this template beside the interactive-artifact snapshot and substitutes its source digest. Deno
 * compilation type-checks both files; this template is not an independently runnable script.
 *
 * Besides the freshness check it shares with the persistent-review entrypoint, it tells the helper where its skill
 * directory is. The helper reads the page's CSS and JS from `assets/` there on every request, and inside the binary
 * `import.meta.url` would name the build snapshot instead.
 */
import { pathToFileURL } from "node:url";

const expected = "__SOURCE_SHA256__";
// Published executables live at <skill>/bin/<platform>/interactive-artifact. Never resolve shipped files from the
// build path.
const skill = new URL("../../", pathToFileURL(Deno.execPath()));
try {
  const source = new URL("interactive-artifact.ts", skill);
  const hash = await crypto.subtle.digest("SHA-256", await Deno.readFile(source));
  const actual = Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, "0")).join("");
  if (actual !== expected) throw new Error("interactive-artifact.ts has changed");
} catch (error) {
  console.error(
    `interactive-artifact: compiled helper is stale or its source is unavailable: ${error}. Rebuild helpers before syncing this skill. No command was executed.`,
  );
  Deno.exit(2);
}
const { runCli } = await import("./interactive-artifact.ts");
await runCli(skill);
