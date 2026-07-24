# Granblue Fantasy: Relink 2.0.2 item-name catalog

- Game executable SHA-256: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`
- Converter: GBFRDataTools 2.0.0
- Converter archive SHA-256: `2F355E7785D7ED7D1A4F99B1FCCC626BB9D949CE29A4F08B816A233DAB77F63B`
- Table conversion version: `2.0.2`
- `system/table/item.tbl` SHA-256: `99D6450E0908F13F3D1E8A76E7A3E66A3E16505D24858F6D0E4D2B6FBD7FA9D0`
- Korean `text.msg` SHA-256: `E03EF29EAC56BB6EAA32EE48D848887F073806F8514A52D31D38F2D9F0397090`
- English `text.msg` SHA-256: `0230DC9A2E42B97C2BFC7B9B6DD074A43F82A661BAA23C69EE8F4A3DA3D0096D`
- Source rows: 448
- Localized output rows per language: 384
- Excluded unnamed internal rows: 64
- Missing Korean/English names among the 384 named rows: 0 / 0

The ordinary-item catalog contains 281 IDs. Of those, 278 have localized names.
`ITEM_33_0004` (`3ca218dd`), `ITEM_33_0001` (`7b3f6dd9`), and
`ITEM_13_0004` (`84f84569`) have an empty `ItemName` in the official 2.0.2 table,
so the application deliberately uses its existing item-ID fallback for those three.

## Reproduction

Set `$dataTool`, `$extractedRoot`, and `$sqlitePath` to local paths outside the
repository. Extract only the required files from a legally obtained local game install:

```powershell
& $dataTool extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/item.tbl' `
  -o $extractedRoot
& $dataTool extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/text/ko/text.msg' `
  -o $extractedRoot
& $dataTool extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/text/en/text.msg' `
  -o $extractedRoot
```

Verify the archive and extracted-file hashes above before conversion:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath $converterArchive
Get-FileHash -Algorithm SHA256 -LiteralPath `
  (Join-Path $extractedRoot 'system\table\item.tbl')
Get-FileHash -Algorithm SHA256 -LiteralPath `
  (Join-Path $extractedRoot 'system\table\text\ko\text.msg')
Get-FileHash -Algorithm SHA256 -LiteralPath `
  (Join-Path $extractedRoot 'system\table\text\en\text.msg')
```

Convert the table and generate both language catalogs in one validated run:

```powershell
& $dataTool tbl-to-sqlite `
  -i (Join-Path $extractedRoot 'system\table') `
  -o $sqlitePath `
  -v 2.0.2

cargo run --locked --release --package gbfr-logs --example build_item_catalog -- `
  $sqlitePath `
  (Join-Path $extractedRoot 'system\table\text\ko\text.msg') `
  (Join-Path $extractedRoot 'system\table\text\en\text.msg') `
  'src-tauri\lang\ko\items.json' `
  'src-tauri\lang\en\items.json' `
  '63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F'
```

Expected generator result:

```text
wrote 384 localized items per language for game 2.0.2; excluded 64 unnamed rows
```

The source game assets and converter binaries are not committed. Catalog generation
does not establish runtime compatibility; the separate offline/private-session smoke
test remains required.
