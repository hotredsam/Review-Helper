import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * IPC contract suite (Phase 15 T6).
 *
 * 16 of 21 frontend test files mock src/api and cargo tests call Rust directly,
 * so nothing else in the repo would notice a renamed command, a dropped
 * registration, or an argument-key mismatch — the exact class of bug behind the
 * dead device-flow commands and the dead Delete-subject button. This suite
 * statically cross-checks every invoke() call site against the Rust side:
 *
 *  1. every invoked command is registered in generate_handler![]
 *  2. every invoked argument key matches a parameter of the Rust fn
 *     (Tauri 2 maps camelCase JS keys to snake_case Rust args)
 *  3. every required (non-Option) Rust parameter is supplied at the call site
 *  4. every registered command is invoked somewhere — or explicitly listed in
 *     EXPECTED_UNINVOKED with a reason
 */

const ROOT = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..");

/** Registered commands with no frontend call site — each needs a reason. */
const EXPECTED_UNINVOKED: Record<string, string> = {
  github_device_start: "device flow built in Phase 3, gh-token path chosen instead — resolve in Phase 18 T5",
  github_device_poll: "device flow built in Phase 3, gh-token path chosen instead — resolve in Phase 18 T5",
  app_info: "build-health probe exercised by cargo test + CI, deliberately not part of the UI",
};

function walk(dir: string, ext: RegExp, skip: RegExp = /node_modules|\/target\//): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (skip.test(p)) continue;
    if (statSync(p).isDirectory()) out.push(...walk(p, ext, skip));
    else if (ext.test(p)) out.push(p);
  }
  return out;
}

const camel = (s: string) => s.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());

/** Split a Rust parameter list on top-level commas (generics-aware). */
function splitParams(raw: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let cur = "";
  for (const ch of raw) {
    if (ch === "<" || ch === "(") depth++;
    else if (ch === ">" || ch === ")") depth--;
    if (ch === "," && depth === 0) {
      parts.push(cur);
      cur = "";
    } else cur += ch;
  }
  if (cur.trim()) parts.push(cur);
  return parts;
}

interface RustCommand {
  file: string;
  params: { name: string; required: boolean }[];
}

/** Parse every #[tauri::command] fn in src-tauri/src. */
function rustCommands(): Map<string, RustCommand> {
  const cmds = new Map<string, RustCommand>();
  for (const file of walk(join(ROOT, "src-tauri", "src"), /\.rs$/)) {
    const src = readFileSync(file, "utf8");
    const re = /#\[tauri::command\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([\s\S]*?)\)\s*(?:->|\{)/g;
    for (let m = re.exec(src); m; m = re.exec(src)) {
      const [, name, rawParams] = m;
      const params = splitParams(rawParams)
        .map((p) => {
          const idx = p.indexOf(":");
          if (idx === -1) return null;
          const pname = p.slice(0, idx).trim().replace(/^mut\s+/, "");
          const ptype = p.slice(idx + 1).trim();
          // Injected by Tauri, not part of the JS-facing contract.
          if (/\b(State|AppHandle|Window|WebviewWindow)\s*</.test(ptype + "<")) return null;
          return { name: pname, required: !ptype.startsWith("Option<") };
        })
        .filter((p): p is { name: string; required: boolean } => p !== null);
      cmds.set(name, { file: file.slice(ROOT.length + 1), params });
    }
  }
  return cmds;
}

/** Command names listed in generate_handler![...] in lib.rs. */
function registeredCommands(): Set<string> {
  const lib = readFileSync(join(ROOT, "src-tauri", "src", "lib.rs"), "utf8");
  const m = lib.match(/generate_handler!\[([\s\S]*?)\]/);
  if (!m) throw new Error("generate_handler![] not found in lib.rs");
  return new Set(
    m[1]
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean)
      .map((s) => s.split("::").pop()!),
  );
}

interface InvokeSite {
  file: string;
  command: string;
  keys: string[] | null; // null = no args object at the call site
}

/** Every invoke("command", {...}) call site in src/ (tests excluded). */
function invokeSites(): InvokeSite[] {
  const sites: InvokeSite[] = [];
  for (const file of walk(join(ROOT, "src"), /\.(ts|tsx)$/, /node_modules|src\/test/)) {
    const src = readFileSync(file, "utf8");
    const re = /\binvoke(?:<[^;]*?>)?\(\s*["'](\w+)["']\s*(?:,\s*\{([\s\S]*?)\}\s*)?\)/g;
    for (let m = re.exec(src); m; m = re.exec(src)) {
      const [, command, rawArgs] = m;
      if (rawArgs === undefined) {
        sites.push({ file: file.slice(ROOT.length + 1), command, keys: null });
        continue;
      }
      const keys: string[] = [];
      for (const part of splitParams(rawArgs)) {
        const t = part.trim();
        if (!t) continue;
        const kv = t.match(/^(\w+)\s*:/);
        if (kv) keys.push(kv[1]);
        else if (/^\w+$/.test(t)) keys.push(t); // shorthand { preview }
      }
      sites.push({ file: file.slice(ROOT.length + 1), command, keys });
    }
  }
  return sites;
}

describe("IPC contract: src/ invoke() ↔ src-tauri commands", () => {
  const registered = registeredCommands();
  const commands = rustCommands();
  const sites = invokeSites();

  it("found a sane amount of both sides (parser self-check)", () => {
    expect(registered.size).toBeGreaterThan(40);
    expect(sites.length).toBeGreaterThan(40);
    // Every registered command has a parsed definition.
    const undefinedCmds = [...registered].filter((c) => !commands.has(c));
    expect(undefinedCmds, `registered but no #[tauri::command] fn parsed: ${undefinedCmds}`).toEqual([]);
  });

  it("every invoked command is registered", () => {
    const ghosts = sites.filter((s) => !registered.has(s.command));
    expect(
      ghosts.map((g) => `${g.file}: invoke("${g.command}")`),
      "invoke() of a command that is not in generate_handler![]",
    ).toEqual([]);
  });

  it("every invoked argument key matches a Rust parameter (camelCase ↔ snake_case)", () => {
    const errors: string[] = [];
    for (const s of sites) {
      const cmd = commands.get(s.command);
      if (!cmd || s.keys === null) continue;
      const expected = new Set(cmd.params.map((p) => camel(p.name)));
      for (const k of s.keys) {
        if (!expected.has(k)) {
          errors.push(`${s.file}: invoke("${s.command}") passes "${k}" — Rust fn (${cmd.file}) expects [${[...expected]}]`);
        }
      }
    }
    expect(errors).toEqual([]);
  });

  it("every required Rust parameter is supplied at each call site", () => {
    const errors: string[] = [];
    for (const s of sites) {
      const cmd = commands.get(s.command);
      if (!cmd) continue;
      const got = new Set(s.keys ?? []);
      for (const p of cmd.params) {
        if (p.required && !got.has(camel(p.name))) {
          errors.push(`${s.file}: invoke("${s.command}") omits required "${camel(p.name)}" (${cmd.file})`);
        }
      }
    }
    expect(errors).toEqual([]);
  });

  it("every registered command is invoked somewhere, or on the expected-dead list", () => {
    const invoked = new Set(sites.map((s) => s.command));
    const dead = [...registered].filter((c) => !invoked.has(c) && !(c in EXPECTED_UNINVOKED));
    expect(dead, "registered but never invoked and not in EXPECTED_UNINVOKED").toEqual([]);
    // And the allowlist must not rot: everything on it really is uninvoked.
    const stale = Object.keys(EXPECTED_UNINVOKED).filter((c) => invoked.has(c));
    expect(stale, "EXPECTED_UNINVOKED entries that are now invoked — remove them").toEqual([]);
  });
});
