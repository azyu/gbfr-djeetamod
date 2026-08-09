# Game 2.0.2 Smoke Test

이 문서는 자동 테스트로 확인할 수 없는 실제 게임 동작을 검증하기 위한 체크리스트입니다. 모든 항목은 먼저 오프라인 또는 비공개 환경에서 수행합니다.

## 환경 기록

- 게임 빌드: 2.0.2 / 실제 표시:
- 화면 해상도:
- 파티 구성:
- NSIS installer SHA-256: `4107A6CD68D18DFD3B1B710243A8F93A2CA8794C29A844BA2CD62151A3F709C8`
- `hook.dll` SHA-256: `80984A0BE803DCE2AA5E5408C4FC22586F06F7FACACAD5DD304E8217ADF39F97`
- 훅 로그 경로:
- 테스트 일시:

## 매니페스트 버전 검사

```powershell
$npmVersion = (Get-Content -Raw package.json | ConvertFrom-Json).version
$tauriVersion = (Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json).package.version
$cargoVersion = (Select-String '^version = "(.+)"' src-tauri/Cargo.toml).Matches[0].Groups[1].Value
$versions = @(@($npmVersion, $tauriVersion, $cargoVersion) | Select-Object -Unique)
if ($versions.Count -ne 1 -or $versions[0] -ne '0.1.5') { throw 'Version mismatch or unexpected release version' }
```

## 자동 업데이트 검증

- 부트스트랩 설치 프로그램 버전/해시:
- 대상 GitHub Release 태그/URL:
- 업데이트 전후 `logs.db`, 설정 및 창 위치 증거:
- 원본 명령 복구 read-back 증거:
- 원격 자산 이름/다이제스트 및 `latest.json` 비교 결과:

- [ ] Updater-enabled `0.1.1` detects stable `0.1.2` and shows its version/notes.
- [ ] **나중에** leaves `0.1.1` running and data unchanged.
- [ ] Offline, missing-manifest, and invalid-signature failures do not block the meter.
- [ ] Installation is refused while `granblue_fantasy_relink.exe` remains running.
- [ ] With repeat quest ON, update preparation reads both sites back as original before prompting for game exit.
- [ ] After installation, the app restarts as `0.1.2`; logs, settings, and window geometry remain present.
- [ ] GitHub Release contains the installer, `.nsis.zip`, `.sig`, and `latest.json` whose URL/signature agree.

## 게임 검증

| 완료 | 시나리오 | 기대 결과 | 실제 결과 |
|---|---|---|---|
| [ ] | 사용자 전용 설치 | 이전 MSI를 Windows 설치된 앱에서 제거한 뒤 NSIS 설치 프로그램이 관리자 권한 상승 없이 실행되고 현재 사용자의 설치된 앱에 Djeeta MOD가 표시된다 | |
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
| [x] | 외부 읽기 전용 진 스냅샷 | 개발 프로브가 파티 4명의 현재 진 스냅샷과 각 캐릭터의 변경 후 스냅샷을 훅 기준값과 동일하게 읽는다 | 2026-07-20: 네 슬롯 모두 변경 전·후 `MATCH` |
| [x] | 외부 리더 재실행 복구 | 게임을 세 번 재실행해도 새 PID에서 고정 해시와 서명을 확인하고 네 슬롯을 다시 찾는다 | PID `6052`, `24620`, `5340`에서 최종 4개 `MATCH` |
| [ ] | 재도전 | 새 전투가 누적 피해 0에서 시작한다 | |

모든 필수 항목에 실제 결과와 통과 여부를 기록하기 전에는 NSIS 설치 프로그램을 게임 2.0.2 호환으로 확정하지 않습니다.

## Game 2.0.4 디버그 빌드 관찰 부록

- 검증일: 2026-08-09
- 실행 파일 SHA-256:
  `F827F3C13CAA90B290FAB2FE7E28165A80448FDE0A3F7A96D79DAC6B8343FF2A`
- 세션: 사용자 확인 오프라인
- 게임 프로세스 수명: 3회(재시작 2회)
- 앱: `0.1.4` 개발 빌드

| 결과 | 시나리오 | 실제 관찰 |
| --- | --- | --- |
| PASS | Hook 주입과 핸드셰이크 | 세 프로세스에서 새 PID를 찾고 Hook 신원과 연결 상태를 다시 수립했다 |
| PASS | 전투 누계와 경계 | 첫 타격, 추가 피해·DoT·SBA·링크 어택, 다중 대상, 보상 직전 초기화, 재도전 초기화를 확인했다 |
| PASS | 장비 분석 | 네 캐릭터의 변경 전·후 스냅샷이 `MATCH`였고 장착 진 12개 범위와 2.0.4 raw-hash 특성명을 확인했다 |
| PASS | 인벤토리와 일반 아이템 | `2,192 -> 2,196` 통제 변경, 정렬·필터 안정성, `197`개 일반 아이템 디코딩을 확인했다 |
| PASS | 무한 퀘스트 반복 | ON에서 정상 열 번 제한을 넘어 열한 번째 이후도 재도전됐고, OFF 후 다음 결과에서 정상 제한이 복구됐다 |
| PASS | 미터 창 정책 | 330x145 최대 4행, OFF/ON, 헤더 이동, 항상 위, 작업표시줄 제외를 확인했다 |
| PASS | 게임 종료 | 무한 반복 ON 상태에서 게임이 정상 종료됐고 앱은 게임-not-found와 비활성 스위치로 전환됐다 |

동일 캐릭터 두 명의 행 분리와 900개 이상 일반 아이템 Windows 알림은
해당 통제 조건이 없어 실행하지 않았다. 이 부록은 개발 빌드의 관찰
증거이며 NSIS 패키지나 Granblue Fantasy: Relink 2.0.4 전체 호환성을
확정하지 않는다.
