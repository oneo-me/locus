import { createPublicKey, verify, createHash } from "node:crypto";
import { mkdir, readFile, writeFile, rm, copyFile, chmod } from "node:fs/promises";
import { join, resolve } from "node:path";

export const SPARKLE_VERSION = "2.10.0";
export const SPARKLE_SHA256 = "c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c";
export const REPOSITORY = "oneo-me/locus";

export function releaseVersion(tag: string, version: string): string {
  if (!/^v0\.[1-9]\d{3}\.[1-9]\d*$/.test(tag) || tag !== `v${version}`
      || version.split(".").some(n => !Number.isSafeInteger(Number(n)))) {
    throw new Error("Expected v0.<year>.<release> matching source/Cargo.toml");
  }
  return version;
}

export function compareVersions(a: string, b: string): number {
  const normalize = (value: string) => {
    const version = value.replace(/^v/, "");
    return releaseVersion(`v${version}`, version).split(".").map(Number);
  };
  const left = normalize(a);
  const right = normalize(b);
  for (let i = 0; i < 3; i++) if (left[i] !== right[i]) return left[i] - right[i];
  return 0;
}

export function releaseNotes(changelog: string, version: string): string {
  const heading = `## [${version}]`;
  const lines = changelog.split("\n");
  const start = lines.findIndex(line => line === heading || line.startsWith(`${heading} - `));
  if (start < 0) throw new Error(`Move shipping Unreleased notes into ${heading} before tagging`);
  let end = start + 1;
  while (end < lines.length && !lines[end].startsWith("## ")) end++;
  const notes = lines.slice(start + 1, end).join("\n").trim();
  if (!notes) throw new Error("Release notes must not be empty");
  return notes;
}

export function verifyArchive(archive: Uint8Array, signature: string, publicKey: string): void {
  const raw = Buffer.from(publicKey, "base64");
  if (raw.length !== 32) throw new Error("SPARKLE_PUBLIC_KEY must be a base64 Ed25519 public key");
  const key = createPublicKey({
    key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), raw]),
    format: "der", type: "spki",
  });
  if (!verify(null, archive, key, Buffer.from(signature, "base64"))) {
    throw new Error("Update signature does not match SPARKLE_PUBLIC_KEY");
  }
}

export function appcast(version: string, signature: string, length: number): string {
  releaseVersion(`v${version}`, version);
  if (!/^[A-Za-z0-9+/]{86}==$/.test(signature) || !Number.isSafeInteger(length) || length <= 0) {
    throw new Error("Invalid update enclosure");
  }
  return `<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Locus updates</title>
    <link>https://github.com/${REPOSITORY}/releases</link>
    <description>Stable macOS releases of Locus</description>
    <item>
      <title>Locus ${version}</title>
      <sparkle:version>${version}</sparkle:version>
      <sparkle:shortVersionString>${version}</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>13.0</sparkle:minimumSystemVersion>
      <enclosure url="https://github.com/${REPOSITORY}/releases/download/v${version}/Locus-${version}-macos-universal.zip"
        sparkle:edSignature="${signature}" length="${length}" type="application/octet-stream" />
    </item>
  </channel>
</rss>
`;
}

type SigningConfiguration =
  | { mode: "ad-hoc"; identity: "-" }
  | { mode: "developer-id"; identity: string; certificate: string; certificatePassword: string;
      appleId: string; applePassword: string; team: string };

export function signingConfiguration(env: Record<string, string | undefined>): SigningConfiguration {
  const mode = env.MACOS_SIGNING_MODE?.trim() || "ad-hoc";
  if (mode === "ad-hoc") return { mode, identity: "-" };
  if (mode !== "developer-id") throw new Error("MACOS_SIGNING_MODE must be ad-hoc or developer-id");
  const identity = required("APPLE_SIGNING_IDENTITY", env);
  if (!identity.startsWith("Developer ID Application:")) throw new Error("A Developer ID Application identity is required");
  return {
    mode, identity,
    certificate: required("APPLE_CERTIFICATE_P12", env),
    certificatePassword: required("APPLE_CERTIFICATE_PASSWORD", env),
    appleId: required("APPLE_ID", env),
    applePassword: required("APPLE_APP_PASSWORD", env),
    team: required("APPLE_TEAM_ID", env),
  };
}

export function signingCommands(config: SigningConfiguration, framework: string, app: string): string[][] {
  // Ad-hoc apps must not enable library validation: there is no Apple team identity.
  const options = config.mode === "ad-hoc"
    ? ["--options", "0", "--timestamp=none"]
    : ["--options", "runtime", "--timestamp"];
  const sign = ["codesign", "--force", ...options, "--sign", config.identity];
  const nested = [
    "Versions/B/XPCServices/Downloader.xpc", "Versions/B/XPCServices/Installer.xpc",
    "Versions/B/Autoupdate", "Versions/B/Updater.app", "",
  ].map(path => [...sign, "--preserve-metadata=identifier,entitlements", join(framework, path)]);
  return [...nested, [...sign, app]];
}

function required(name: string, env: Record<string, string | undefined> = process.env): string {
  const value = env[name]?.trim();
  if (!value) throw new Error(`Missing ${name}`);
  return value;
}

function run(args: string[], options: { input?: string; capture?: boolean } = {}): string {
  const result = Bun.spawnSync(args, {
    stdin: options.input === undefined ? "ignore" : Buffer.from(options.input),
    stdout: options.capture ? "pipe" : "inherit", stderr: "inherit",
  });
  if (result.exitCode !== 0) throw new Error(`${args[0]} failed (${result.exitCode})`);
  return options.capture ? result.stdout.toString().trim() : "";
}

async function latestVersion(token: string): Promise<string | undefined> {
  const response = await fetch(`https://api.github.com/repos/${REPOSITORY}/releases/latest`, {
    headers: { Authorization: `Bearer ${token}`, Accept: "application/vnd.github+json" },
  });
  if (response.status === 404) return undefined;
  if (!response.ok) throw new Error(`Cannot check latest release (${response.status})`);
  return (await response.json() as { tag_name: string }).tag_name;
}

async function release() {
  if (process.platform !== "darwin") throw new Error("Releases require macOS");
  if (required("GITHUB_REPOSITORY") !== REPOSITORY) throw new Error("Release repository does not match the update feed");
  const metadata = JSON.parse(run(["cargo", "metadata", "--manifest-path", "source/Cargo.toml", "--locked", "--no-deps", "--format-version", "1"], { capture: true }));
  const version = releaseVersion(required("GITHUB_REF_NAME"), metadata.packages.find((p: { name: string }) => p.name === "locus").version);
  const notes = releaseNotes(await readFile("CHANGELOG.md", "utf8"), version);
  const token = required("GH_TOKEN");
  const latest = await latestVersion(token);
  if (latest && compareVersions(version, latest) <= 0) throw new Error("Release must be newer than the published latest version");
  const existingResponse = await fetch(`https://api.github.com/repos/${REPOSITORY}/releases/tags/v${version}`, {
    headers: { Authorization: `Bearer ${token}`, Accept: "application/vnd.github+json" },
  });
  let existingDraft = false;
  if (existingResponse.ok) {
    existingDraft = (await existingResponse.json() as { draft: boolean }).draft;
    if (!existingDraft) throw new Error("Published releases are immutable; use a new version");
  } else if (existingResponse.status !== 404) {
    throw new Error(`Cannot check existing release (${existingResponse.status})`);
  }
  const publicKey = required("SPARKLE_PUBLIC_KEY");
  const privateKey = required("SPARKLE_PRIVATE_KEY");
  const signing = signingConfiguration(process.env);
  console.log(`macOS signing mode: ${signing.mode}`);
  const directory = resolve(required("RUNNER_TEMP"), "locus-release");
  await mkdir(directory, { recursive: true });
  const keychain = join(directory, "signing.keychain-db");
  const p12 = join(directory, "certificate.p12");
  const password = crypto.randomUUID();
  let previousKeychains: string[] | undefined;
  try {
    if (signing.mode === "developer-id") {
      previousKeychains = run(["security", "list-keychains", "-d", "user"], { capture: true }).match(/"([^"]+)"/g)?.map(s => s.slice(1, -1)) ?? [];
      await writeFile(p12, Buffer.from(signing.certificate, "base64"), { mode: 0o600 });
      run(["security", "create-keychain", "-p", password, keychain]);
      run(["security", "set-keychain-settings", "-lut", "21600", keychain]);
      run(["security", "unlock-keychain", "-p", password, keychain]);
      run(["security", "import", p12, "-k", keychain, "-P", signing.certificatePassword, "-T", "/usr/bin/codesign", "-T", "/usr/bin/security"]);
      run(["security", "set-key-partition-list", "-S", "apple-tool:,apple:,codesign:", "-s", "-k", password, keychain]);
      run(["security", "list-keychains", "-d", "user", "-s", keychain, ...previousKeychains]);
    }
    const download = await fetch(`https://github.com/sparkle-project/Sparkle/releases/download/${SPARKLE_VERSION}/Sparkle-${SPARKLE_VERSION}.tar.xz`);
    if (!download.ok) throw new Error("Could not download Sparkle");
    const bytes = Buffer.from(await download.arrayBuffer());
    if (createHash("sha256").update(bytes).digest("hex") !== SPARKLE_SHA256) throw new Error("Sparkle checksum mismatch");
    const archive = join(directory, "sparkle.tar.xz");
    await writeFile(archive, bytes);
    const sparkle = join(directory, "sparkle");
    await mkdir(sparkle);
    run(["tar", "-xf", archive, "-C", sparkle]);
    const app = resolve("dist/Locus.app");
    for (const target of ["aarch64-apple-darwin", "x86_64-apple-darwin"]) {
      run(["cargo", "run", "--manifest-path", "source/Cargo.toml", "--locked", "-p", "oneo-tool-packer", "--", "--project", "source", "--target", target]);
      await copyFile(join(app, "Contents/MacOS/locus"), join(directory, target));
    }
    run(["lipo", "-create", join(directory, "aarch64-apple-darwin"), join(directory, "x86_64-apple-darwin"), "-output", join(app, "Contents/MacOS/locus")]);
    run(["lipo", "-verify_arch", "arm64", "x86_64", join(app, "Contents/MacOS/locus")]);
    await chmod(join(app, "Contents/MacOS/locus"), 0o755);
    const framework = join(app, "Contents/Frameworks/Sparkle.framework");
    await mkdir(join(app, "Contents/Frameworks"), { recursive: true });
    run(["ditto", join(sparkle, "Sparkle.framework"), framework]);
    const plist = join(app, "Contents/Info.plist");
    for (const [key, type, value] of [
      ["SUFeedURL", "string", `https://github.com/${REPOSITORY}/releases/latest/download/appcast.xml`],
      ["SUPublicEDKey", "string", publicKey],
      ["SUVerifyUpdateBeforeExtraction", "bool", "true"],
      ["SUEnableAutomaticChecks", "bool", "true"],
      ["SUAutomaticallyUpdate", "bool", "true"],
      ["SUScheduledCheckInterval", "integer", "86400"],
    ]) run(["/usr/libexec/PlistBuddy", "-c", `Add :${key} ${type} ${value}`, plist]);
    // Sign nested code inside-out, rather than using codesign --deep to sign.
    for (const command of signingCommands(signing, framework, app)) run(command);
    run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", app]);
    const zip = resolve(`dist/Locus-${version}-macos-universal.zip`);
    const zipApp = () => run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", app, zip]);
    zipApp();
    if (signing.mode === "developer-id") {
      run(["xcrun", "notarytool", "submit", zip, "--apple-id", signing.appleId, "--password", signing.applePassword, "--team-id", signing.team, "--wait", "--timeout", "30m"]);
      run(["xcrun", "stapler", "staple", app]);
      run(["xcrun", "stapler", "validate", app]);
      run(["spctl", "--assess", "--type", "execute", "--verbose=2", app]);
      await rm(zip);
      zipApp();
    }
    const signature = run([join(sparkle, "bin/sign_update"), "--ed-key-file", "-", "-p", zip], { input: privateKey, capture: true });
    const finalBytes = await readFile(zip);
    verifyArchive(finalBytes, signature, publicKey);
    await writeFile("dist/appcast.xml", appcast(version, signature, finalBytes.length));
    run(["xmllint", "--noout", "dist/appcast.xml"]);
    await writeFile("dist/SHA256SUMS", `${createHash("sha256").update(finalBytes).digest("hex")}  Locus-${version}-macos-universal.zip\n`);
    const notesFile = join(directory, "notes.md");
    const installationNotice = signing.mode === "ad-hoc"
      ? "This macOS app is ad-hoc signed, not Apple-notarized. On first installation, macOS may block opening it. Only if you trust this release, allow it in System Settings → Privacy & Security after attempting to open it. Sparkle update archives are independently signed with Ed25519.\n\n"
      : "";
    await writeFile(notesFile, installationNotice + notes);
    // A draft keeps /releases/latest unchanged until all assets are uploaded.
    const tag = `v${version}`;
    if (existingDraft) {
      run(["gh", "release", "edit", tag, "--title", `Locus ${version}`, "--notes-file", notesFile]);
    } else {
      run(["gh", "release", "create", tag, "--verify-tag", "--draft", "--title", `Locus ${version}`, "--notes-file", notesFile]);
    }
    run(["gh", "release", "upload", tag, zip, "dist/appcast.xml", "dist/SHA256SUMS", "--clobber"]);
    run(["gh", "release", "edit", tag, "--draft=false", "--latest"]);
  } finally {
    try {
      if (previousKeychains) run(["security", "list-keychains", "-d", "user", "-s", ...previousKeychains]);
    } finally {
      // Delete credentials even if restoring the keychain search list fails.
      if (previousKeychains) Bun.spawnSync(["security", "delete-keychain", keychain], { stdout: "ignore", stderr: "ignore" });
      await rm(directory, { recursive: true, force: true });
    }
  }
}

if (import.meta.main) await release();
