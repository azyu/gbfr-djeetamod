# Game 2.0.2 Smoke Test

이 문서는 자동 테스트로 확인할 수 없는 실제 게임 동작을 검증하기 위한 체크리스트입니다. 모든 항목은 먼저 오프라인 또는 비공개 환경에서 수행합니다.

## 환경 기록

- 게임 빌드: 2.0.2 / 실제 표시:
- 화면 해상도:
- 파티 구성:
- MSI SHA-256: `7C65BF53D78E34A656D6D0A9B922DF3B0552EC5A6E797F915AEBB7A5E8F2F92A`
- `hook.dll` SHA-256: `09AC0D0757F0BA31188FED10ACCDE11166068D76480EA4F7C7A5DCB689EA231B`
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
| [ ] | 1920x1080 창 초기화 | 미터가 330x145, x45/y470의 4행 고정 크기로 표시되며 파티와 전투 HUD를 가리지 않는다 | |
| [ ] | 데미지 미터 스위치 | 왼쪽 메뉴의 스위치를 끄면 미터가 사라지고 켜면 같은 위치에 다시 나타난다 | |
| [ ] | 미터 위치 이동 | `파티 데미지` 헤더를 드래그하면 고정 크기를 유지한 채 이동하며 스크롤바가 생기지 않는다 | |
| [ ] | 작업 표시줄 정책 | `Djeeta MOD` 관리 창만 작업 표시줄에 나타나고 미터는 별도 항목을 만들지 않는다 | |
| [ ] | 창 항상 위 정책 | 미터는 항상 위에 유지되고 관리 창의 항상 위 설정은 기본적으로 꺼져 있다 | |
| [ ] | 나루메아 진 분석 | 장비 편성에서 나루메아를 열면 데미지 상한 `70 / 65`와 `5 초과`가 표시된다 | |
| [ ] | Endless Ragnarok 공식 특성명 | 새 특성 하나 이상이 게임 UI와 같은 공식 한국어 이름으로 표시된다 | |
| [ ] | 영어 특성명 | 앱 언어를 영어로 바꾸면 같은 특성이 공식 영어 이름으로 표시된다 | |
| [ ] | 이름 미확인 특성 | raw hash 또는 이름 행이 없는 특성을 만났을 때 `알 수 없는 특성 (0x1234abcd)` 형식의 8자리 ID가 표시되고 화면이 중단되지 않는다 | |
| [ ] | 진 분석 범위 | 화면과 README 설명이 장착 진 12개의 주·보조 특성만 합산하며 무기·가호석·소환석·마스터 특성은 아직 제외됨을 명확히 한다 | |
| [ ] | 재도전 | 새 전투가 누적 피해 0에서 시작한다 | |

모든 필수 항목에 실제 결과와 통과 여부를 기록하기 전에는 MSI를 게임 2.0.2 호환으로 확정하지 않습니다.
