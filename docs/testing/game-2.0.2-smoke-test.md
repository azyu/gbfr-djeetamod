# Game 2.0.2 Smoke Test

이 문서는 자동 테스트로 확인할 수 없는 실제 게임 동작을 검증하기 위한 체크리스트입니다. 모든 항목은 먼저 오프라인 또는 비공개 환경에서 수행합니다.

## 환경 기록

- 게임 빌드: 2.0.2 / 실제 표시:
- 화면 해상도:
- 파티 구성:
- MSI SHA-256: `A56AC9C15ACEBD8F75B64CFE59D42C9047351B140731D61815722E97D4A4B9FA`
- `hook.dll` SHA-256: `F9AF8931603C7FFE466E0FB5726882C2377C57BB1ECA8FFBA433F21AA2EA41EA`
- 훅 로그 경로:
- 테스트 일시:

## 매니페스트 버전 검사

```powershell
$npmVersion = (Get-Content -Raw package.json | ConvertFrom-Json).version
$tauriVersion = (Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json).package.version
$cargoVersion = (Select-String '^version = "(.+)"' src-tauri/Cargo.toml).Matches[0].Groups[1].Value
$versions = @(@($npmVersion, $tauriVersion, $cargoVersion) | Select-Object -Unique)
if ($versions.Count -ne 1 -or $versions[0] -ne '0.1.0') { throw 'Version mismatch or unexpected release version' }
```

## 게임 검증

| 완료 | 시나리오 | 기대 결과 | 실제 결과 |
|---|---|---|---|
| [ ] | 훈련장 단일 타격 | 화면 피해와 누적 피해가 일치한다 | |
| [ ] | 서로 다른 4캐릭터 | 네 행이 캐릭터별로 분리된다 | |
| [ ] | 온라인 동일 캐릭터 2명 | 두 actor가 별도 행으로 유지된다 | |
| [ ] | 추가 피해, DoT, SBA, 링크 어택 | 중복 또는 누락 없이 합산된다 | |
| [ ] | 적 2마리 퀘스트 | 두 대상 피해가 한 전투 총합에 포함된다 | |
| [ ] | 마지막 적 처치 후 결과 전환 | reward UI 직전까지 유지되고 진입 전에 사라진다 | |
| [ ] | 전투 중 게임 종료 | 미터가 닫히고 게임과 앱이 충돌하지 않는다 | |
| [ ] | 1920x1080 창 초기화 | 330x145, x45/y470이며 파티와 전투 HUD를 가리지 않는다 | |
| [ ] | 클릭 통과 전환 | 설정 모드에서 이동·크기 변경, 일반 모드에서 게임 입력 통과 | |
| [ ] | 재도전 | 새 전투가 누적 피해 0에서 시작한다 | |

모든 필수 항목에 실제 결과와 통과 여부를 기록하기 전에는 MSI를 게임 2.0.2 호환으로 확정하지 않습니다.
