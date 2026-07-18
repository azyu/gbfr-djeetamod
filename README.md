# GBFR Korean Damage Meter

Granblue Fantasy: Relink Endless Ragnarok 2.0.2를 대상으로 개발 중인 Windows x64 파티 데미지 미터 테스트 빌드입니다. 캐릭터별 누적 피해, 상대 비율 바와 DPS를 한국어 소형 오버레이로 표시합니다.

> 현재 자동 테스트와 MSI 패키징은 완료됐지만, 실제 게임 2.0.2 플레이 체크리스트는 아직 검증되지 않았습니다. 아래 MSI를 호환성이 확정된 정식 릴리스로 간주하지 마십시오.

## 설치와 실행

1. 릴리스의 MSI를 설치합니다.
2. 게임을 먼저 실행합니다.
3. GBFR Korean Damage Meter를 실행합니다.
4. 트레이 메뉴의 클릭 통과를 끄면 화면 창을 이동하거나 크기를 조절할 수 있고, 다시 켜면 입력이 게임으로 전달됩니다.

제거는 Windows의 설치된 앱에서 수행합니다. 사용자 설정과 로그는 `%AppData%` 아래 애플리케이션 데이터 폴더에 남을 수 있습니다.

## 표시와 동작

- 기본 언어는 한국어입니다.
- 1920x1080 기준 크기는 330x145이며 화면 왼쪽의 파티 HUD 아래쪽에 배치됩니다.
- 전투 첫 피해부터 캐릭터별 누적 피해와 DPS를 표시합니다.
- 수치는 해당 전투의 보상 화면이 열리기 직전까지 유지됩니다.
- 게임 2.0.2용 필수 훅을 찾지 못하면 설정 화면에 연결 오류를 표시합니다.

## 주의

이 도구는 DLL 주입, 게임 메모리 읽기와 런타임 코드 패치를 사용합니다. Cygames가 공식 허용하거나 화이트리스트에 등록한 도구가 아니며, 온라인 사용과 계정 제재 위험이 없다고 보증하지 않습니다. 먼저 오프라인 또는 비공개 환경에서 사용하십시오. 게임 업데이트 뒤에는 호환되지 않거나 충돌할 수 있습니다.

백신 프로그램이 동작 방식 때문에 파일을 오탐할 수 있습니다. 출처와 아래 SHA-256 값을 직접 확인한 뒤 설치 여부를 판단하십시오.

## 0.1.0 빌드 해시

- MSI: `598FEA6D228EEEF9BD9EDE1BA33C99588A3D28ACDB40E8B03C41AA014A92570F`
- `hook.dll`: `3D5D83B34E7B476D0A107985B8218B72B6F293D8EAA88E5F207F8978FFADD68B`

## 소스 빌드

Node.js 20, Visual Studio 2022 C++ Build Tools, Windows SDK, WebView2, rustup과 `rust-toolchain.toml`에 지정된 툴체인이 필요합니다.

```powershell
npm ci
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm test -- --run
npm run tauri build -- --bundles msi
```

실제 게임 검증 항목은 [`docs/testing/game-2.0.2-smoke-test.md`](docs/testing/game-2.0.2-smoke-test.md)를 따릅니다.

## 크레딧과 라이선스

MIT 라이선스의 [`false-spring/gbfr-logs`](https://github.com/false-spring/gbfr-logs)와 [`onelittlechildawa/gbfr-logs`](https://github.com/onelittlechildawa/gbfr-logs) Awa Edition 1.8.6을 기반으로 합니다. 전체 저작권 및 허가문은 [`LICENSE`](LICENSE)를 참조하십시오.
