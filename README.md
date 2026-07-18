# Djeeta MOD

Granblue Fantasy: Relink Endless Ragnarok 2.0.2를 대상으로 개발 중인 Windows x64 파티 데미지 미터 테스트 빌드입니다. 캐릭터별 누적 피해, 상대 비율 바와 DPS를 한국어 소형 오버레이로 표시합니다.

> 현재 자동 테스트와 MSI 패키징은 완료됐지만, 실제 게임 2.0.2 플레이 체크리스트는 아직 검증되지 않았습니다. 아래 MSI를 호환성이 확정된 정식 릴리스로 간주하지 마십시오.

## 설치와 실행

1. 릴리스의 MSI를 설치합니다.
2. 게임을 먼저 실행합니다.
3. Djeeta MOD를 실행합니다.
4. 관리 창의 왼쪽 메뉴에서 데미지 미터를 켜거나 끌 수 있고, 미터의 상단 제목 영역을 드래그하면 위치를 옮길 수 있습니다.

제거는 Windows의 설치된 앱에서 수행합니다. 사용자 설정과 로그는 `%AppData%` 아래 애플리케이션 데이터 폴더에 남을 수 있습니다.

## 표시와 동작

- 기본 언어는 한국어입니다.
- 1920x1080 기준 크기는 330x145이며 화면 왼쪽의 파티 HUD 아래쪽에 배치됩니다.
- 전투 첫 피해부터 캐릭터별 누적 피해와 DPS를 표시합니다.
- 수치는 해당 전투의 보상 화면이 열리기 직전까지 유지됩니다.
- 게임 2.0.2용 필수 훅을 찾지 못하면 설정 화면에 연결 오류를 표시합니다.

## 사용자 가이드 (한국어)

### 시작하기

1. MSI를 설치한 뒤 Granblue Fantasy: Relink를 먼저 실행합니다.
2. Djeeta MOD를 실행합니다. 작업 표시줄에는 관리 창만 표시되고, 데미지 미터는 게임 위에 별도 창으로 표시됩니다.
3. 관리 창의 왼쪽 사이드바에서 기능을 선택합니다. 데스크톱에서는 사이드바가 기본으로 열려 있습니다.

### 왼쪽 메뉴

- **데미지 미터:** 스위치로 미터를 표시하거나 숨깁니다. 미터 상단의 `파티 데미지` 영역을 마우스로 드래그하면 위치를 옮길 수 있습니다. 미터는 항상 위에 표시되며 최대 4명의 누적 데미지와 DPS를 보여줍니다.
- **진 특성 상한 분석:** 게임에서 읽은 캐릭터별 장착 진 특성을 합산합니다. 현재는 장착 진 12개의 주·보조 특성만 포함합니다. 캐릭터 선택 상자에서 확인할 캐릭터를 바꿀 수 있습니다.
- **전투 기록:** 완료된 전투의 저장 기록을 열어 파티 데미지, DPS와 세부 기록을 확인합니다.
- **설정:** 언어, 미터 투명도, 표시 항목과 기타 미터 옵션을 변경합니다.

### 진 특성 상한 읽기

- `현재 / 최대`는 장착 진에서 합산한 현재 스킬 레벨과 확인된 상한을 뜻합니다.
- `정상`은 상한 미만, `최대`는 상한 도달, `N 초과`는 상한보다 N레벨 높다는 뜻입니다. 예를 들어 데미지 상한이 `70 / 65`이면 5레벨이 초과된 상태입니다.
- `최대치 미확인`은 해당 특성의 상한 데이터가 아직 검증되지 않았다는 뜻이며, 상한이 없다는 의미가 아닙니다.
- `기여 진`을 펼치면 합계에 포함된 각 진의 주·보조 특성 레벨을 확인할 수 있습니다.

### 문제가 있을 때

- 관리 창이 보이지 않으면 작업 표시줄 또는 시스템 트레이에서 Djeeta MOD를 확인하고, 계속 보이지 않으면 앱을 다시 실행합니다.
- 데미지 미터가 보이지 않으면 왼쪽 메뉴의 **데미지 미터** 스위치를 켭니다.
- `게임 연결 대기 중`이면 게임을 먼저 실행했는지 확인합니다.
- `게임 2.0.2 훅을 찾을 수 없습니다`가 표시되면 게임 버전이 2.0.2인지 확인하고, 백신이 `hook.dll`을 격리하지 않았는지 확인한 뒤 앱을 다시 설치합니다.
- 진 정보가 갱신되지 않으면 게임의 장비 편성 화면에서 확인할 캐릭터의 장비를 열거나 캐릭터를 다시 선택합니다.

`hook.dll`은 게임에서 전투 및 장착 진 정보를 읽어 Djeeta MOD로 전달하는 필수 구성요소입니다. 삭제하거나 격리하면 데미지 미터와 진 특성 분석이 동작하지 않습니다. 이 도구는 공식 허용 도구가 아니므로 먼저 오프라인 또는 비공개 환경에서 테스트하십시오.

## User Guide (English)

### Getting started

1. Install the MSI, then start Granblue Fantasy: Relink first.
2. Start Djeeta MOD. Only the management window appears on the taskbar; the damage meter is a separate overlay above the game.
3. Select a feature from the left sidebar in the management window. The sidebar is open by default on desktop.

### Left menu

- **Damage Meter:** Use the switch to show or hide the meter. Drag the `Party Damage` header to move it. The meter stays on top and shows cumulative damage and DPS for up to four players.
- **Sigil Trait Cap Analysis:** Totals the equipped sigil traits read from the game for each character. The current scope includes only primary and secondary traits from the 12 equipped sigils. Use the character selector to inspect another character.
- **Battle Records:** Opens saved encounters with party damage, DPS, and detailed records.
- **Settings:** Changes the language, meter transparency, displayed columns, and other meter options.

### Reading sigil trait caps

- `Current / Max` is the total current skill level from equipped sigils and the verified cap.
- `Normal` means below the cap, `Max` means the cap is reached, and `N over` means the total exceeds the cap by N levels. For example, Damage Cap at `70 / 65` is 5 levels over the cap.
- `Maximum unverified` means the cap data for that trait has not been verified yet; it does not mean the trait has no cap.
- Expand `Contributing sigils` to see the primary and secondary trait levels included in the total.

### Troubleshooting

- If the management window is missing, check the taskbar or system tray for Djeeta MOD. Restart the app if it still does not appear.
- If the damage meter is hidden, enable the **Damage Meter** switch in the left menu.
- If the app says `Waiting for the game`, confirm that the game was started first.
- If the app says `The game 2.0.2 hook was not found`, confirm that the game is version 2.0.2, check whether antivirus software quarantined `hook.dll`, and then reinstall the app.
- If sigil information does not update, open the equipment screen for that character in the game or select the character again.

`hook.dll` is required to read combat and equipped-sigil information from the game and deliver it to Djeeta MOD. Removing or quarantining it disables the damage meter and sigil trait analysis. This is not an officially approved tool, so test it in an offline or private session first.

## 성능 영향

Djeeta MOD는 게임의 그래픽 설정이나 렌더링 품질을 변경하지 않습니다. 다만 게임 프로세스의 피해 훅, named pipe 파싱, 별도 투명 WebView 오버레이가 CPU와 메모리를 사용합니다. 오버레이 표시는 250ms 간격으로 갱신되고 WebView GPU 가속은 비활성화되어 있어 예상 GPU 부담은 작지만, 실제 게임 비교 측정 전에는 성능 영향이 전혀 없다고 보증하지 않습니다.

## 주의

이 도구는 DLL 주입, 게임 메모리 읽기와 런타임 코드 패치를 사용합니다. Cygames가 공식 허용하거나 화이트리스트에 등록한 도구가 아니며, 온라인 사용과 계정 제재 위험이 없다고 보증하지 않습니다. 먼저 오프라인 또는 비공개 환경에서 사용하십시오. 게임 업데이트 뒤에는 호환되지 않거나 충돌할 수 있습니다.

백신 프로그램이 동작 방식 때문에 파일을 오탐할 수 있습니다. 출처와 아래 SHA-256 값을 직접 확인한 뒤 설치 여부를 판단하십시오.

## 0.1.0 빌드 해시

- MSI: `F8C5612A0903CB753E112A98065BDA72CDFFC2CC3BE9FEB5926F532DF519CC31`
- `hook.dll`: `09AC0D0757F0BA31188FED10ACCDE11166068D76480EA4F7C7A5DCB689EA231B`

## 소스 빌드

Node.js 20, Visual Studio 2022 C++ Build Tools, Windows SDK, WebView2, rustup과 `rust-toolchain.toml`에 지정된 툴체인이 필요합니다.

전체 검증, 최신 훅 동기화, MSI 생성과 해시 기록 갱신은 다음 명령으로 실행합니다. 게임은 먼저 종료해야 합니다.

```powershell
npm run package:msi
```

스크립트가 실행하는 개별 검증 명령은 다음과 같습니다.

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm run tauri build -- --bundles msi
```

실제 게임 검증 항목은 [`docs/testing/game-2.0.2-smoke-test.md`](docs/testing/game-2.0.2-smoke-test.md)를 따릅니다.

## 크레딧과 라이선스

MIT 라이선스의 [`false-spring/gbfr-logs`](https://github.com/false-spring/gbfr-logs)와 [`onelittlechildawa/gbfr-logs`](https://github.com/onelittlechildawa/gbfr-logs) Awa Edition 1.8.6을 기반으로 합니다. 전체 저작권 및 허가문은 [`LICENSE`](LICENSE)를 참조하십시오.
