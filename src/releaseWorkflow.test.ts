import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

it("publishes only verified manually dispatched signed releases", () => {
  const workflow = readFileSync(resolve(process.cwd(), ".github/workflows/release.yaml"), "utf8");

  expect(workflow).toContain("workflow_dispatch:");
  expect(workflow).toContain("contents: write");
  expect(workflow).toContain("environment: production");
  expect(workflow).toContain("TAURI_PRIVATE_KEY");
  expect(workflow).toContain("TAURI_KEY_PASSWORD");
  expect(workflow).toContain("git tag --list $tag");
  expect(workflow).toContain("gh release list --limit 1000 --json tagName");
  expect(workflow).not.toContain('git rev-parse "refs/tags/$tag"');
  expect(workflow).not.toContain("gh release view $tag");
  expect(workflow).toContain("npm.cmd run package:nsis -- -RequestedVersion");
  expect(workflow).toContain("gh release create");
  expect(workflow).toContain("--draft");
  expect(workflow).toContain('gh api "repos/${{ github.repository }}/releases?per_page=100"');
  expect(workflow).not.toContain("releases/tags/$tag");
  expect(workflow).toContain("ConvertTo-GitHubReleaseAssetName");
  expect(workflow).toContain("latest.json");
  expect(workflow).toContain("gh release edit");
  expect(workflow).not.toMatch(/uses:\s+[^\s]+@(v\d+|main|master)\b/);
});
