import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

it("publishes only verified manually dispatched signed releases", () => {
  const workflow = readFileSync(resolve(process.cwd(), ".github/workflows/release.yaml"), "utf8");
  const signingStep = workflow.match(/- name: Sign updater archive[\s\S]*?(?=\r?\n {6}- name:|\s*$)/)?.[0];

  expect(workflow).toContain("workflow_dispatch:");
  expect(workflow).toContain("contents: write");
  expect(workflow).toContain("environment: production");
  expect(workflow).toContain("ref: ${{ github.sha }}");
  expect(workflow).not.toContain("ref: main");
  expect(workflow).toContain("persist-credentials: false");
  expect(workflow).not.toMatch(/^\s{4}env:\s*\r?\n(?:\s{6}.+\r?\n)*\s{6}TAURI_PRIVATE_KEY:/m);
  expect(workflow).not.toContain("$version = '${{ inputs.version }}'");
  expect(workflow).not.toContain("'${{ inputs.publish }}'");
  expect(workflow).toContain("RELEASE_VERSION: ${{ inputs.version }}");
  expect(workflow).toContain("PUBLISH_RELEASE: ${{ inputs.publish }}");
  expect(workflow).toContain("$version = $env:RELEASE_VERSION");
  expect(workflow).toContain("TAURI_PRIVATE_KEY");
  expect(workflow).toContain("TAURI_KEY_PASSWORD");
  expect(workflow).toContain("git tag --list $tag");
  expect(workflow).toContain("gh release list --limit 1000 --json tagName");
  expect(workflow).not.toContain('git rev-parse "refs/tags/$tag"');
  expect(workflow).not.toContain("gh release view $tag");
  expect(workflow).toContain("npm.cmd run package:nsis -- -RequestedVersion");
  expect(workflow).toContain("npm.cmd run package:sign -- -RequestedVersion");
  expect(workflow).toContain("gh auth setup-git");
  expect(workflow).toContain("gh release create");
  expect(workflow).toContain("--draft");
  expect(workflow).toContain('gh api "repos/${{ github.repository }}/releases?per_page=100"');
  expect(workflow).not.toContain("releases/tags/$tag");
  expect(workflow).toContain("for ($attempt = 1; $attempt -le 5; $attempt++)");
  expect(workflow).toContain("Start-Sleep -Seconds 2");
  expect(workflow).toContain("ConvertTo-GitHubReleaseAssetName");
  expect(workflow).toContain("latest.json");
  expect(workflow).toContain("gh release edit");
  expect(workflow).not.toMatch(/uses:\s+[^\s]+@(v\d+|main|master)\b/);

  expect(signingStep).toBeDefined();
  expect(signingStep).toContain("TAURI_PRIVATE_KEY");
  expect(signingStep).toContain("TAURI_KEY_PASSWORD");
  expect(signingStep).not.toContain("npm ci");
  expect(signingStep).not.toContain("cargo");
  expect(signingStep).not.toContain("package:nsis");
  expect(signingStep).not.toContain("git push");
  expect(signingStep).not.toContain("gh release");
});
