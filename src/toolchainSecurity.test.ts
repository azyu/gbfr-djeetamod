import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const repositoryPath = (path: string) => resolve(process.cwd(), path);
const readRepositoryFile = (path: string) => readFileSync(repositoryPath(path), "utf8");

it("declares Node.js 24 and npm 11 as the supported JavaScript toolchain", () => {
  const nvmrcPath = repositoryPath(".nvmrc");
  expect(existsSync(nvmrcPath)).toBe(true);
  if (!existsSync(nvmrcPath)) return;

  const packageJson = JSON.parse(readRepositoryFile("package.json")) as {
    engines?: { node?: string };
    packageManager?: string;
  };

  expect(readFileSync(nvmrcPath, "utf8").trim()).toBe("24");
  expect(packageJson.engines).toEqual({ node: ">=24.0.0 <25" });
  expect(packageJson.packageManager).toBe("npm@11.13.0");
  expect(readRepositoryFile("scripts/PackageHelpers.psm1")).toContain("Node.js 24 is required");
  expect(readRepositoryFile("README.md")).toContain("Node.js 24");
});

it("uses Node.js 24 and immutable third-party actions in CI", () => {
  const ci = readRepositoryFile(".github/workflows/ci.yaml");
  const release = readRepositoryFile(".github/workflows/release.yaml");

  expect(ci.match(/node-version: 24/g) ?? []).toHaveLength(4);
  expect(release).toContain("node-version: 24");
  expect(ci).not.toMatch(/uses:\s+actions\/checkout@v\d+/);
  expect(ci).not.toMatch(/uses:\s+actions\/setup-node@v\d+/);
  expect(ci).not.toMatch(/uses:\s+Swatinem\/rust-cache@v\d+/);
  expect(ci).not.toContain("rustup update nightly");
  expect(ci).toContain("actions/checkout@11d5960a326750d5838078e36cf38b85af677262");
  expect(ci).toContain("actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020");
  expect(ci).toContain("Swatinem/rust-cache@e18b497796c12c097a38f9edb9d0641fb99eee32");
  expect(ci).toContain("rustup toolchain install nightly-2025-06-27 --profile minimal");
});

it("uses patched Vite and Vitest versions and audits them in CI", () => {
  const packageJson = JSON.parse(readRepositoryFile("package.json")) as {
    scripts: Record<string, string>;
    devDependencies: Record<string, string>;
  };
  const ci = readRepositoryFile(".github/workflows/ci.yaml");

  expect(packageJson.scripts["audit:npm"]).toBe("npm audit");
  expect(packageJson.devDependencies).toMatchObject({
    "@types/node": "^24.0.0",
    "@vitejs/plugin-react": "^4.7.0",
    vite: "^6.4.2",
    vitest: "^4.1.10",
  });
  expect(ci).toContain("npm run audit:npm");
});

it("pins a Rust 1.88-capable nightly and audits both lockfiles", () => {
  const packageJson = JSON.parse(readRepositoryFile("package.json")) as {
    scripts: Record<string, string>;
  };
  const auditScriptPath = repositoryPath("scripts/audit-rust.ps1");
  const ci = readRepositoryFile(".github/workflows/ci.yaml");

  expect(readRepositoryFile("rust-toolchain.toml")).toContain('channel = "nightly-2025-06-27"');
  expect(packageJson.scripts["audit:rust"]).toBe(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/audit-rust.ps1"
  );
  expect(existsSync(auditScriptPath)).toBe(true);
  expect(ci).toContain("npm run audit:rust");
  if (!existsSync(auditScriptPath)) return;

  const auditScript = readFileSync(auditScriptPath, "utf8");
  expect(auditScript).toContain("cargo-audit-x86_64-pc-windows-msvc-v0.22.2.zip");
  expect(auditScript).toContain("0a7316540862c13d954f648917ceacca593747baed6eec180fafa590be2710ab");
  expect(auditScript).toContain("Get-FileHash");
  expect(auditScript).toContain("Cargo.lock");
  expect(auditScript).toContain("protocol\\Cargo.lock");
  expect(auditScript).toContain("audit --no-fetch --file 'protocol\\Cargo.lock'");
});

it("uses a current dll-syringe without unused RPC and cross-bitness features", () => {
  expect(readRepositoryFile("src-tauri/Cargo.toml")).toContain(
    'dll-syringe = { version = "0.17.1", default-features = false, features = ["syringe"] }'
  );
});
