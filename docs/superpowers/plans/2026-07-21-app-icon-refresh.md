# App Icon Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace every Djeeta MOD bundle icon with a face-focused crop of the supplied artwork and use the same icon for the NSIS installer.

**Architecture:** Produce one 1024×1024 canonical PNG from a fixed square crop of the 1254×1254 source, then let the Tauri 1 icon generator derive all platform-specific assets. Keep runtime code unchanged and verify the generated assets and final NSIS bundle independently.

**Tech Stack:** PowerShell `System.Drawing`, Tauri CLI 1, PNG/ICO/ICNS, NSIS

## Global Constraints

- Preserve the supplied artwork and white background; do not use AI regeneration or visual retouching.
- Use the fixed 900×900 crop at source coordinates x=170, y=100, then resize it to 1024×1024 with high-quality bicubic interpolation.
- Keep `logs.db` untracked and untouched.
- Preserve the existing uncommitted packaging-hash changes until the final package refreshes them.
- Do not change runtime, window, hook, protocol, or game compatibility behavior.

---

### Task 1: Generate and configure the icon family

**Files:**
- Modify: `src-tauri/icons/app-icon.png`
- Modify: `src-tauri/icons/32x32.png`
- Modify: `src-tauri/icons/128x128.png`
- Modify: `src-tauri/icons/128x128@2x.png`
- Modify: `src-tauri/icons/icon.ico`
- Modify: `src-tauri/icons/icon.icns`
- Modify: `src-tauri/icons/icon.png`
- Modify: `src-tauri/icons/Square*.png`
- Modify: `src-tauri/icons/StoreLogo.png`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: `C:\Users\azyu\Downloads\image.png`, a 1254×1254 RGBA PNG.
- Produces: `src-tauri/icons/app-icon.png`, the 1024×1024 canonical source, and the Tauri-generated platform icon family.

- [ ] **Step 1: Record the pre-change failure conditions**

Run:

```powershell
Add-Type -AssemblyName System.Drawing
$config = Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json
$source = [System.Drawing.Image]::FromFile('C:\Users\azyu\Downloads\image.png')
$current = [System.Drawing.Image]::FromFile('src-tauri\icons\app-icon.png')
"source=$($source.Width)x$($source.Height); current=$($current.Width)x$($current.Height); installerIcon=$($config.tauri.bundle.windows.nsis.installerIcon)"
$source.Dispose()
$current.Dispose()
```

Expected: the current canonical icon is not the requested 1024×1024 crop and `installerIcon` is empty.

- [ ] **Step 2: Create the canonical face-focused PNG**

Run a PowerShell `System.Drawing` conversion that:

```powershell
Add-Type -AssemblyName System.Drawing
$sourcePath = 'C:\Users\azyu\Downloads\image.png'
$outputPath = 'src-tauri\icons\app-icon.png'
$source = [System.Drawing.Bitmap]::FromFile($sourcePath)
$output = New-Object System.Drawing.Bitmap 1024, 1024, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$graphics = [System.Drawing.Graphics]::FromImage($output)
$graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$destination = New-Object System.Drawing.Rectangle 0, 0, 1024, 1024
$crop = New-Object System.Drawing.Rectangle 170, 100, 900, 900
$graphics.DrawImage($source, $destination, $crop, [System.Drawing.GraphicsUnit]::Pixel)
$output.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$output.Dispose()
$source.Dispose()
```

Expected: `src-tauri/icons/app-icon.png` is a 1024×1024 PNG centered on the face, goggles, hair, and neck decoration.

- [ ] **Step 3: Generate all Tauri icons from the canonical PNG**

Run:

```powershell
npm.cmd run tauri -- icon src-tauri/icons/app-icon.png --output src-tauri/icons
```

Expected: the command succeeds and refreshes the PNG, ICO, ICNS, Windows tile, and store assets in `src-tauri/icons`.

- [ ] **Step 4: Configure the NSIS installer icon**

Modify `src-tauri/tauri.conf.json`:

```json
"nsis": {
  "installMode": "currentUser",
  "installerIcon": "icons/icon.ico"
}
```

Expected: the installed executable, shortcuts, and installer executable all use the refreshed icon family.

- [ ] **Step 5: Verify generated formats and configuration**

Run:

```powershell
Add-Type -AssemblyName System.Drawing
$required = @('app-icon.png', '32x32.png', '128x128.png', '128x128@2x.png', 'icon.ico', 'icon.icns', 'StoreLogo.png')
$required | ForEach-Object { if (-not (Test-Path (Join-Path 'src-tauri\icons' $_))) { throw "Missing icon: $_" } }
$canonical = [System.Drawing.Image]::FromFile('src-tauri\icons\app-icon.png')
if ($canonical.Width -ne 1024 -or $canonical.Height -ne 1024) { throw 'Canonical icon must be 1024x1024' }
$canonical.Dispose()
$config = Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json
if ($config.tauri.bundle.windows.nsis.installerIcon -ne 'icons/icon.ico') { throw 'NSIS installerIcon is not configured' }
git diff --check
```

Expected: no exception and no `git diff --check` output.

- [ ] **Step 6: Visually inspect the canonical and 32×32 icons**

Open `src-tauri/icons/app-icon.png` and `src-tauri/icons/32x32.png` with the image viewer.

Expected: the crop preserves the face, goggles, hair silhouette, and neck decoration; the 32×32 icon remains recognizable and is not distorted.

- [ ] **Step 7: Commit the icon family**

```powershell
git add -- src-tauri/icons src-tauri/tauri.conf.json
git commit -m "feat: refresh app icon artwork"
```

Expected: only icon assets and `tauri.conf.json` are included in this commit.

### Task 2: Build and verify the NSIS distribution

**Files:**
- Modify: `README.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Output: `target/release/bundle/nsis/Djeeta MOD_0.1.0_x64-setup.exe`

**Interfaces:**
- Consumes: the refreshed `src-tauri/icons/icon.ico` and existing `scripts/package.ps1` packaging pipeline.
- Produces: a verified NSIS installer plus refreshed installer and hook SHA-256 documentation.

- [ ] **Step 1: Run the complete packaging verification**

Run:

```powershell
npm.cmd run package:nsis
```

Expected: package helper tests, `npm ci`, format check, lint, TypeScript check, 72 frontend tests, frontend build, release hook build, all Rust workspace tests, Tauri NSIS build, hook copy verification, and documentation hash update all succeed. The existing Node 24 warning is informational; Node 20 remains the project standard.

- [ ] **Step 2: Verify the final artifacts and hashes**

Run:

```powershell
$installer = 'target\release\bundle\nsis\Djeeta MOD_0.1.0_x64-setup.exe'
if (-not (Test-Path $installer)) { throw 'NSIS installer missing' }
$builtHook = (Get-FileHash 'target\release\hook.dll' -Algorithm SHA256).Hash
$bundledHook = (Get-FileHash 'src-tauri\hook.dll' -Algorithm SHA256).Hash
if ($builtHook -ne $bundledHook) { throw 'hook.dll hashes differ' }
Get-Item $installer | Select-Object FullName, Length
Get-FileHash $installer, 'target\release\hook.dll' -Algorithm SHA256
git diff --check
```

Expected: the installer exists, both hook hashes are equal, hashes print successfully, and `git diff --check` emits no errors.

- [ ] **Step 3: Review the final worktree scope**

Run:

```powershell
git status --short
git diff -- README.md docs/testing/game-2.0.2-smoke-test.md
```

Expected: packaging documentation contains the new artifact hashes; `logs.db` remains the only unrelated untracked file.

- [ ] **Step 4: Commit refreshed distribution metadata**

```powershell
git add -- README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: record refreshed app installer hashes"
```

Expected: only the two packaging documentation files are included.
