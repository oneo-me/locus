import { describe, expect, test } from "bun:test";
import { generateKeyPairSync, sign } from "node:crypto";
import { appcast, compareVersions, releaseNotes, releaseVersion, signingCommands, signingConfiguration, verifyArchive } from "./release";

describe("release contracts", () => {
  const appleCredentials = {
    APPLE_SIGNING_IDENTITY: "Developer ID Application: Example (TEAM)",
    APPLE_CERTIFICATE_P12: "certificate", APPLE_CERTIFICATE_PASSWORD: "password",
    APPLE_ID: "developer@example.com", APPLE_APP_PASSWORD: "app-password", APPLE_TEAM_ID: "TEAM",
  };
  test("defaults to ad-hoc without any Apple account or credentials", () => {
    expect(signingConfiguration({})).toEqual({ mode: "ad-hoc", identity: "-" });
    expect(signingConfiguration({ MACOS_SIGNING_MODE: "" }).mode).toBe("ad-hoc");
    // Stale Apple secrets never opt a release into signing implicitly.
    expect(signingConfiguration(appleCredentials).mode).toBe("ad-hoc");
  });
  test("Developer ID mode is explicit and fails closed on incomplete credentials", () => {
    const config = signingConfiguration({ MACOS_SIGNING_MODE: "developer-id", ...appleCredentials });
    expect(config.mode).toBe("developer-id");
    for (const name of Object.keys(appleCredentials)) {
      expect(() => signingConfiguration({ MACOS_SIGNING_MODE: "developer-id", ...appleCredentials, [name]: "" })).toThrow();
    }
    expect(() => signingConfiguration({ MACOS_SIGNING_MODE: "unknown" })).toThrow();
    expect(() => signingConfiguration({
      MACOS_SIGNING_MODE: "developer-id", ...appleCredentials, APPLE_SIGNING_IDENTITY: "Apple Development: Example",
    })).toThrow();
  });
  test("ad-hoc signing clears hardened runtime and uses no timestamp service", () => {
    const commands = signingCommands(signingConfiguration({}), "/App/Sparkle.framework", "/App");
    expect(commands).toHaveLength(6);
    expect(commands.at(-1)?.at(-1)).toBe("/App");
    expect(commands[0].at(-1)).toEndWith("Downloader.xpc");
    for (const command of commands) {
      expect(command[command.indexOf("--sign") + 1]).toBe("-");
      expect(command[command.indexOf("--options") + 1]).toBe("0");
      expect(command).toContain("--timestamp=none");
      expect(command).not.toContain("runtime");
    }
  });
  test("Developer ID signing enables hardened runtime and trusted timestamps", () => {
    const config = signingConfiguration({ MACOS_SIGNING_MODE: "developer-id", ...appleCredentials });
    for (const command of signingCommands(config, "/App/Sparkle.framework", "/App")) {
      expect(command[command.indexOf("--sign") + 1]).toBe(appleCredentials.APPLE_SIGNING_IDENTITY);
      expect(command).toContain("runtime");
      expect(command).toContain("--timestamp");
    }
  });
  test("only matching stable calendar-version tags are accepted", () => {
    expect(releaseVersion("v0.2026.1", "0.2026.1")).toBe("0.2026.1");
    for (const tag of ["0.2026.1", "v0.2026.2", "v0.2026.1-beta", "v1.2026.1", "v0.2026.0", "v0.2026.01"]) {
      expect(() => releaseVersion(tag, "0.2026.1")).toThrow();
    }
  });
  test("orders versions numerically, including year rollover", () => {
    expect(compareVersions("0.2026.10", "v0.2026.9")).toBeGreaterThan(0);
    expect(compareVersions("0.2027.1", "v0.2026.99")).toBeGreaterThan(0);
    expect(compareVersions("0.2026.1", "v0.2026.1")).toBe(0);
    expect(compareVersions("0.2026.1", "v0.2026.2")).toBeLessThan(0);
    for (const invalid of ["invalid", "0..1", "0.2026.-1", "0.2026.01", "0.2026.99999999999999999999"]) {
      expect(() => compareVersions("0.2026.1", invalid)).toThrow();
    }
  });
  test("extracts only confirmed release notes", () => {
    const changelog = "# Changelog\n\n## [Unreleased]\n- Later\n\n## [0.2026.2] - 2026-09-01\n\n- Shipping\n\n## [0.2026.1]\n- Old\n";
    expect(releaseNotes(changelog, "0.2026.2")).toBe("- Shipping");
    expect(() => releaseNotes(changelog, "0.2026.3")).toThrow();
    expect(() => releaseNotes("## [0.2026.2]\n\n## [0.2026.1]", "0.2026.2")).toThrow();
  });
  test("verifies the final archive against the public key embedded in the app", () => {
    const { publicKey, privateKey } = generateKeyPairSync("ed25519");
    const bytes = Buffer.from("final release archive");
    const rawKey = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("base64");
    const signature = sign(null, bytes, privateKey).toString("base64");
    expect(() => verifyArchive(bytes, signature, rawKey)).not.toThrow();
    expect(() => verifyArchive(Buffer.from("modified"), signature, rawKey)).toThrow();
    const otherKey = generateKeyPairSync("ed25519").publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("base64");
    expect(() => verifyArchive(bytes, signature, otherKey)).toThrow();
    expect(() => verifyArchive(bytes, signature, "invalid")).toThrow();
  });
  test("uses immutable universal archive URLs and signed enclosures", () => {
    const signature = Buffer.alloc(64).toString("base64");
    const xml = appcast("0.2026.2", signature, 12345);
    expect(xml).toContain("/releases/download/v0.2026.2/Locus-0.2026.2-macos-universal.zip");
    expect(xml).toContain('length="12345"');
    expect(xml).toContain(`sparkle:edSignature="${signature}"`);
    expect(xml).toContain("<sparkle:minimumSystemVersion>13.0</sparkle:minimumSystemVersion>");
    expect(() => appcast("0.2026.2", '"><bad/>', 12345)).toThrow();
    expect(() => appcast("0.2026.2", signature, 0)).toThrow();
  });
});
