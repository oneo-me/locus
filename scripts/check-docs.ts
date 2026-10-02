// Run from any directory with Bun; no package installation is required.
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";

const ROOT = resolve(import.meta.dir, "..");
const OVERVIEW = "ARCHITECTURE.md";
const SUBJECTS = "docs/architecture";
const REQUIRED = ["AGENTS.md", OVERVIEW, "README.md"];

function* proseLines(markdown: string) {
  let fence: { character: string; length: number } | undefined;
  for (const [index, line] of markdown.split(/\r?\n/).entries()) {
    const marker = /^\s{0,3}(`{3,}|~{3,})(.*)$/.exec(line);
    if (marker) {
      const [, run, tail] = marker;
      if (!fence) {
        fence = { character: run[0], length: run.length };
      } else if (
        run[0] === fence.character && run.length >= fence.length && !tail.trim()
      ) {
        fence = undefined;
      }
      continue;
    }
    if (!fence) yield { number: index + 1, line };
  }
}

export function anchors(markdown: string) {
  const seen = new Map<string, number>();
  const result = new Set<string>();
  let previous = "";
  for (const { line } of proseLines(markdown)) {
    const heading = /^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$/.exec(line);
    let text = heading?.[1];
    if (!text && previous.trim() && /^\s{0,3}(?:=+|-+)\s*$/.test(line)) {
      text = previous.trim();
    }
    previous = line;
    if (!text) continue;
    const slug = text
      .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/<[^>]*>/g, "")
      .replace(/`/g, "")
      .toLowerCase()
      .replace(/[^\p{L}\p{N}\s_-]/gu, "")
      .replace(/\s/g, "-");
    let candidate = slug;
    while (result.has(candidate)) {
      const count = (seen.get(slug) ?? 0) + 1;
      seen.set(slug, count);
      candidate = `${slug}-${count}`;
    }
    result.add(candidate);
  }
  return result;
}

export function* links(markdown: string) {
  for (const { number, line } of proseLines(markdown)) {
    const prose = line.replace(/(`+).*?\1/g, "");
    const inline = /!?\[[^\]\n]*\]\(\s*(?:<([^>\n]+)>|([^\s)]+))(?:\s+["'][^\n]*?["'])?\s*\)/g;
    for (const match of prose.matchAll(inline)) {
      yield { number, target: match[1] ?? match[2] };
    }
    const reference = /^\s{0,3}\[[^\]]+\]:\s*(?:<([^>]+)>|(\S+))/.exec(prose);
    if (reference) yield { number, target: reference[1] ?? reference[2] };
  }
}

function localTarget(target: string) {
  if (/^[a-z][\w+.-]*:/i.test(target) || target.startsWith("/")) return;
  const [file, ...fragment] = target.split("#");
  return {
    file: decodeURIComponent(file.split("?")[0]),
    anchor: decodeURIComponent(fragment.join("#")).toLowerCase(),
  };
}

export function checkDocs(directory = ROOT) {
  const root = realpathSync(directory);
  const problems: string[] = [];
  const texts = new Map<string, string>();
  const read = (path: string) => {
    if (!texts.has(path)) texts.set(path, readFileSync(path, "utf8").replace(/\r\n/g, "\n"));
    return texts.get(path)!;
  };
  const scan = (pattern: string) => [...new Bun.Glob(pattern).scanSync({ cwd: root, onlyFiles: true })];
  const docs = new Set([...REQUIRED, ...scan("docs/**/*.md"), ...scan("source/resources/skills/**/*.md")]);

  for (const doc of [...docs].sort()) {
    const source = join(root, doc);
    if (!existsSync(source) || !statSync(source).isFile()) {
      problems.push(`${doc}: required document missing`);
      continue;
    }
    for (const { number, target } of links(read(source))) {
      const label = `${doc}:${number}: ${target}`;
      let local;
      try {
        local = localTarget(target);
      } catch {
        problems.push(`${label}: invalid URL encoding`);
        continue;
      }
      if (!local) continue;
      const path = local.file ? resolve(dirname(source), local.file) : source;
      if (!existsSync(path)) {
        problems.push(`${label}: target missing`);
      } else if (local.anchor && extname(path).toLowerCase() === ".md") {
        if (!statSync(path).isFile() || !anchors(read(path)).has(local.anchor)) {
          problems.push(`${label}: heading missing`);
        }
      }
    }
  }

  const overview = join(root, OVERVIEW);
  const subjects = new Set(scan(`${SUBJECTS}/*.md`).map((path) => join(root, path)));
  if (!subjects.size) problems.push(`${SUBJECTS}: no architecture subjects`);
  if (existsSync(overview) && statSync(overview).isFile()) {
    const section = /^## 专题\s*\n([\s\S]*?)(?=^#{1,2} |$(?![\s\S]))/m.exec(read(overview));
    const listed = new Map<string, number>();
    if (section) {
      for (const { target } of links(section[1])) {
        let local;
        try {
          local = localTarget(target);
        } catch {
          continue; // Reported above with its source line.
        }
        if (!local) {
          problems.push(`${OVERVIEW}: subject table contains an external link: ${target}`);
          continue;
        }
        const path = resolve(root, local.file);
        listed.set(path, (listed.get(path) ?? 0) + 1);
      }
    } else {
      problems.push(`${OVERVIEW}: subject section missing`);
    }
    for (const path of subjects) {
      if (!listed.has(path)) problems.push(`${relative(root, path)}: absent from subject table`);
    }
    for (const [path, count] of listed) {
      if (!subjects.has(path)) problems.push(`${OVERVIEW}: subject table contains a non-subject: ${path}`);
      if (count > 1) problems.push(`${OVERVIEW}: subject listed more than once: ${relative(root, path)}`);
    }
  }

  const budgeted: [string, number][] = [[overview, 16 * 1024]];
  budgeted.push(...[...subjects].sort().map((path): [string, number] => [path, 24 * 1024]));
  for (const [path, budget] of budgeted) {
    if (!existsSync(path) || !statSync(path).isFile()) continue;
    const size = Buffer.byteLength(read(path), "utf8");
    console.log(`${relative(root, path)}: ${size.toLocaleString("en-US")} / ${budget.toLocaleString("en-US")} bytes`);
    if (size > budget) problems.push(`${relative(root, path)}: exceeds reading budget`);
  }
  for (const problem of problems) console.error(problem);
  if (!problems.length) {
    console.log(`Documentation check passed: ${docs.size} documents, ${subjects.size} subjects.`);
  }
  return problems.length ? 1 : 0;
}

if (import.meta.main) process.exitCode = checkDocs();
