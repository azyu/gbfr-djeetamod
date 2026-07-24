# Relink Modding 사이트 참고 정리

작성일: 2026-07-24

대상 사이트: [Granblue Fantasy: Relink Modding](https://nenkai.github.io/relink-modding/)

확인한 원본: [`nenkai/relink-modding` 커밋 `57f7b1d`](https://github.com/nenkai/relink-modding/commit/57f7b1dad7bb909704c96effdaa52ef2b0c51dca)

## 목적과 사용 원칙

이 문서는 Relink Modding 사이트의 전체 내용을 복제하지 않고, Djeeta MOD 개발 중 어떤 자료를 어디서 찾아야 하는지 빠르게 판단하기 위한 한국어 색인이다. 세부 ID, 표, 구조체 필드와 도구 명령은 계속 바뀔 수 있으므로 실제 작업에서는 링크된 원문과 현재 게임 파일을 다시 확인한다.

특히 다음 경계를 지킨다.

- 사이트의 오래된 시그니처, 주소, 구조 설명은 연구 단서일 뿐 Endless Ragnarok 2.0.2 호환성의 증거가 아니다.
- 실행 파일 시그니처와 메모리 레이아웃은 Djeeta MOD가 고정한 2.0.2 실행 파일에서 별도로 검증한다.
- 테이블 스키마는 버전 업데이트 때 열이 이동할 수 있다. 변환할 때 정확한 게임 버전을 지정하고, 현재 `GBFRDataTools` 헤더와 추출 결과를 기준으로 삼는다.
- 표 편집이나 코드 주입 기능은 오프라인 또는 비공개 세션에서만 검증한다. 사이트도 테이블 MOD의 온라인 사용과 치트성 배포를 경고한다.
- 사이트의 API·네트워크 페이지는 프로토콜 연구 자료다. Djeeta MOD의 현재 로컬 훅/파이프 구조에 필요하지 않은 서비스 키나 인증 구현은 이 문서에 옮기지 않았다.
- 이 자료만으로 게임 2.0.2 호환성을 주장하지 않는다. 호환성 판정은 `docs/testing/game-2.0.2-smoke-test.md`의 수동 검증을 따른다.

## Djeeta MOD에서 먼저 볼 페이지

| 작업                       | 우선 참고할 페이지                                                                                                                                                                                                                                                   | 활용 범위                                                                             |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| 게임 파일과 테이블 추출    | [파일 추출](https://nenkai.github.io/relink-modding/tutorials/file_extraction/), [테이블 DB](https://nenkai.github.io/relink-modding/tables/table_database/)                                                                                                         | `data.i`에서 필요한 파일만 추출하고 `.tbl`을 SQLite로 변환                            |
| 특성·장비 이름과 상한 조사 | [Trait/Skill IDs](https://nenkai.github.io/relink-modding/resources/trait_skill_ids/), [테이블 목록](https://nenkai.github.io/relink-modding/tables/table_list/), [Hashes](https://nenkai.github.io/relink-modding/resources/re/hashes/)                             | `skill`, `skill_status`, `weapon*`, 아이템/진 테이블과 사용자 정의 XXHash32 관계 확인 |
| 플레이어 피해·스킬 조사    | [Actions / Parameters](https://nenkai.github.io/relink-modding/resources/player/action_parameter/), [Action IDs](https://nenkai.github.io/relink-modding/resources/player/action_ids/), [Motions](https://nenkai.github.io/relink-modding/resources/player/motions/) | 캐릭터별 액션, 모션, 재사용 대기시간, 피해 상한 인덱스의 정적 데이터 파악             |
| 캐릭터·적 식별             | [Model IDs](https://nenkai.github.io/relink-modding/resources/model_ids/), [Entity Prefixes](https://nenkai.github.io/relink-modding/resources/entity_prefixes/), [Obj IDs](https://nenkai.github.io/relink-modding/resources/re/obj_id/)                            | `pl`, `em` 등 모델 접두사와 런타임 객체 ID 범주 구분                                  |
| 전투·퀘스트 경계 조사      | [Quest Base Info](https://nenkai.github.io/relink-modding/resources/quests_layouts/quest_base_info/), [Placement Info](https://nenkai.github.io/relink-modding/resources/quests_layouts/placement/), [FSMs](https://nenkai.github.io/relink-modding/resources/fsm/)  | 퀘스트 목표, 보상, 대상, 배치와 FSM 연결 관계 파악                                    |
| 보상 UI·반복 퀘스트 조사   | [Reverse-Engineering Misc](https://nenkai.github.io/relink-modding/resources/re/misc/)                                                                                                                                                                               | 결과 보상 컨트롤러와 반복 횟수 검사에 관한 이름·단서 확인                             |
| 버전 판별                  | [User Defined Params](https://nenkai.github.io/relink-modding/resources/re/user_attributes/)                                                                                                                                                                         | 실행 파일 리소스의 애플리케이션/표시 버전 구조 확인                                   |
| 업데이트 영향 평가         | [Endless Ragnarok and Mods](https://nenkai.github.io/relink-modding/modding/endless_ragnarok_and_mods/)                                                                                                                                                              | 컴파일러, 테이블 열, 모델 소재 변경으로 깨지는 MOD 유형 확인                          |

## Endless Ragnarok에서 가장 중요한 경고

사이트의 2026-07-14 갱신 내용은 “게임 업데이트 뒤에는 MOD 종류별로 다시 검증해야 한다”는 점을 강조한다.

- 코드 주입 MOD는 컴파일러 변경 때문에 일반적으로 깨졌다. 기존 시그니처가 맞아 보이더라도 동작과 호출 규약을 다시 검증해야 한다.
- 테이블/게임플레이 MOD는 열 이동 때문에 대부분 그대로 호환되지 않는다. 구버전 SQLite나 헤더를 2.0.2에 재사용하면 안 된다.
- 소재 파일을 바꾸는 모델 MOD는 Endless Ragnarok용 갱신이 필요할 수 있다.
- `GBFRDataTools`는 Endless Ragnarok용 파일 목록과 테이블 헤더가 갱신되었다. 다만 일부 헤더는 치트 제작 방지를 위해 공개되지 않는다.
- 일반 MOD 로더의 호환성 설명은 Djeeta MOD의 자체 주입 DLL 호환성을 대신 증명하지 않는다.

이 경고는 현재 프로젝트의 `HookStatus::Unsupported` 래치, 실행 파일 해시 고정, 수동 smoke test 정책과 같은 방향이다.

## 파일 아카이브와 추출

게임 설치 폴더의 핵심 파일은 다음과 같다.

- `data.i`: 번호가 붙은 데이터 아카이브의 인덱스
- `data.0`, `data.1` 등: 실제 게임 콘텐츠 아카이브
- `data/`: 주로 외부 사운드 파일

사이트 설명에 따르면 `data.i`는 FlatBuffers 형식이고, 파일 경로는 XXHash64, 압축은 LZ4를 사용한다. 경로 문자열 자체가 아카이브에 들어 있지 않아 `GBFRDataTools`의 `filelist.txt`에 알려진 경로만 안정적으로 추출할 수 있다.

필요한 파일 하나만 추출하는 기본 형태는 다음과 같다.

```powershell
GBFRDataTools.exe extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/skill_status.tbl' `
  -o '<temporary-output>'
```

전체 추출은 오래 걸리고 알려지지 않은 경로는 빠질 수 있으므로 Djeeta MOD 조사에서는 `extract` 또는 필터가 있는 `extract-all`을 우선한다. 게임 파일은 읽기 전용 입력으로 취급하고 결과는 새 임시 디렉터리에 둔다.

## 테이블, 메시지와 해시

`system/table` 아래 `.tbl`에는 아이템, 능력, 진, 무기, 보상 등 많은 정적 데이터가 있다. 사이트는 `GBFRDataTools`로 SQLite 왕복 변환하는 흐름을 안내한다.

```powershell
GBFRDataTools.exe tbl-to-sqlite `
  -i '<extracted-system-table-directory>' `
  -o '<output.sqlite>' `
  -v 2.0.2
```

조사할 때 기억할 점:

- 열의 실제 형식과 알려진 이름은 도구 배포본의 `Headers`/`.headers` 자료를 확인한다.
- 관계 키처럼 보이는 무작위 32비트 값은 사이트가 설명한 사용자 정의 XXHash32일 수 있다.
- 빈 문자열의 사용자 정의 XXHash32 값은 `0x887AE0B0`이다.
- 파일 경로에 쓰이는 XXHash64와 테이블 관계에 쓰이는 사용자 정의 XXHash32를 혼동하지 않는다.
- `.msg`는 주로 MessagePack이며 텍스트와 여러 게임 데이터에 사용된다.
- SQLite를 다시 `.tbl`로 만드는 기능도 있지만, Djeeta MOD의 카탈로그 생성에는 읽기 전용 변환만 필요하다.

Djeeta MOD와 관련성이 큰 표:

- `skill`: 특성/스킬 정의
- `skill_status`: 특성 레벨별 값
- `ability`: UI에 표시되는 캐릭터 능력
- `weapon`, `weapon_skill_level`, `weapon_status*`: 무기와 레벨/각성/동기화 상태
- `quest_difficulty`, `quest_rank`, `quest_param`: 퀘스트 난이도와 매개변수
- `reward`, `reward_lot`, `reward_quest_rank`: 보상과 보상 그룹
- `result_box_rate`: 결과 보상 상자 관련 표
- `quest_overkill_table`: 보스 체력 초과 피해와 MSP 배율 관련 표

진행 중인 특성 이름/상한 생성의 구체적인 2.0.2 절차는 `docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md`가 이 일반 색인보다 우선한다.

## 플레이어 액션과 피해 조사

사이트는 캐릭터마다 `system/player/data/{model_id}` 아래에 `{model_id}_action`과 `{model_id}_parameter`가 있다고 설명한다.

- parameter 데이터에는 식별 번호가 붙은 피해 상한 배열이 있다.
- action 데이터에는 액션 ID, 연결 모션, 스킬 태그, 재사용 대기시간, 분기 입력, AI 거리/범주, 피해 상한 인덱스 등이 있다.
- 일부 공격은 캐릭터 본체가 아니라 소환 무기나 엔티티가 히트박스를 처리한다. 예로 오이겐의 수류탄, 로제타의 장미, 카타리나의 아레스, 시에테의 아바타, 칼리오스트로의 공격, 산달폰의 보석, 페리의 펫이 언급된다.
- projectile/소환 공격은 액션 파일뿐 아니라 캐릭터 FSM과 생성 엔티티까지 함께 봐야 피해 소유자를 정확히 추적할 수 있다.

이는 Djeeta MOD의 “검증된 플레이어 신원이 소유한 알 수 없는 액터만 허용” 정책에 유용한 정적 단서다. 하지만 이 파일 설명만으로 런타임 actor 포인터, owner 필드 또는 피해 콜백 레이아웃을 확정해서는 안 된다.

## 퀘스트, 보상과 반복 경계

`quest/{quest_id}/baseinfo.msg`는 퀘스트 분류, 난이도, 제한 시간, 적, 목표, 보상, FSM 연결 정보를 담는다. `layout/p{phaseId}/placement_*.msg`는 적, 오브젝트, 보물, 루트와 구역을 배치하는 장면 그래프 역할을 한다.

Djeeta MOD에 특히 유용한 연결은 다음과 같다.

- `baseinfo.msg`의 퀘스트 ID와 표시 적 목록은 전투/퀘스트 로그 분류의 보조 자료가 될 수 있다.
- `targetList_`, 보상 정보와 FSM 연결은 “전투 종료”, “퀘스트 완료”, “보상 UI 진입”이 서로 다른 경계임을 보여준다.
- `rewardRank_`와 보상 문자열은 `reward`/`reward_lot` 테이블로 이어진다.
- Placement의 `EnemySet`, `Enemy`, `Player`, `Zone`, `Behavior` 유형은 다중 대상 전투에서 액터가 어느 범주인지 조사할 때 참고할 수 있다.

[Reverse-Engineering Misc](https://nenkai.github.io/relink-modding/resources/re/misc/)에는 현재 반복 퀘스트 기능과 직접 관련된 두 단서가 있다.

- `ui::component::ControllerResultReward`의 vtable `+0xE8` 호출이 보상 UI를 닫을 때 반복 횟수를 1 증가시킨다는 설명
- `ui::fsm::condition::ResultRetryCountCheck::Execute`가 반복 횟수 `< 10`을 검사한다는 설명

이름과 제어 흐름을 찾는 출발점으로는 유용하지만 주소나 vtable 슬롯의 2.0.2 유효성을 별도 검증해야 한다.

## 런타임 역공학 자료의 해석

사이트의 역공학 영역은 다음 자료를 제공한다.

- [API](https://nenkai.github.io/relink-modding/resources/re/api/): 부팅 시 HTTP API, 뉴스/약관, ER 인증·presence, 퀘스트 종료 후 playlog 개요
- [Hashes](https://nenkai.github.io/relink-modding/resources/re/hashes/): 사용자 정의 XXHash32 알고리즘과 역해시 목록
- [Networking](https://nenkai.github.io/relink-modding/resources/re/networking/): Steam Networking Sockets 앞단 암복호화와 관련 함수 단서
- [Obj IDs](https://nenkai.github.io/relink-modding/resources/re/obj_id/): 플레이어·적·무기·효과 등 런타임 객체 ID 상위 범주
- [Save Unit IDs](https://nenkai.github.io/relink-modding/resources/re/save_units/): 저장 데이터가 UserData, Character, Weapon, Ability, Gem, Item, Quest 등의 manager/type 번호로 나뉜다는 불완전한 목록
- [User Defined Params](https://nenkai.github.io/relink-modding/resources/re/user_attributes/): 실행 파일의 16바이트 리소스에 언어, 애플리케이션 버전, 표시 버전, Granite 타일셋 수가 들어간다는 설명

현재 Djeeta MOD는 로컬 프로세스 안의 훅과 named pipe를 사용하므로 원격 API나 패킷 복호화가 핵심 경로는 아니다. 온라인 서비스·네트워크 연구보다 로컬 actor 소유권, 피해 콜백, 보상 UI 경계를 우선한다.

## 전체 사이트 색인

### 설치와 제작

- [Installing Mods](https://nenkai.github.io/relink-modding/modding/installing_mods/): Reloaded-II/Relink Mod Manager 설치와 수동 설치
- [Creating Mods](https://nenkai.github.io/relink-modding/modding/creating_mods/): Mod Manager용 자산 배치, `.minfo` 버전 보정, `.json`→`.msg` 자동 변환, 배포 시 버전 표기 지침
- [Mod Manager API](https://nenkai.github.io/relink-modding/modding/mod_manager_api/): C# MOD에서 쓰는 Mod Manager API
- [Recommended Mods/Tools](https://nenkai.github.io/relink-modding/modding/recommended_mods_tools/): 관련 도구와 MOD 목록

Djeeta MOD는 독립 Tauri 앱과 자체 훅을 사용하므로 설치 절차를 그대로 따르지는 않는다. 다만 다른 DLL 로더와 충돌하거나 사용자가 게임 폴더에 남긴 `winmm.dll`, `dinput8.dll`, `version.dll`이 문제를 일으키는지 진단할 때 참고할 수 있다.

### 자산 제작

- [Asset Paths](https://nenkai.github.io/relink-modding/resources/asset_paths/): 모델, 효과, 레이아웃, 퀘스트, 시스템, UI 자산 트리
- [File Extensions](https://nenkai.github.io/relink-modding/resources/file_extensions/): `.bxm`, `.msg`, `.tbl`, `.minfo`, `.mmat`, `.mmesh`, `.mot`, Granite texture 등 형식과 도구
- [Textures](https://nenkai.github.io/relink-modding/tutorials/textures/texture_extraction/), [Texture Creation](https://nenkai.github.io/relink-modding/tutorials/textures/texture_creation/): UI/모델 텍스처 추출과 재구성
- [Audio](https://nenkai.github.io/relink-modding/tutorials/audio/audio_extraction/), [Audio Creation](https://nenkai.github.io/relink-modding/tutorials/audio/audio_creation/): Wwise 음원 재생·변환·제작
- [Blender Import](https://nenkai.github.io/relink-modding/models/importing/), [Blender Export](https://nenkai.github.io/relink-modding/models/exporting/), [ER Model Update](https://nenkai.github.io/relink-modding/models/updating_models_for_er/): 모델 작업 흐름

### 식별자와 정적 데이터

- 일반 ID: [Item](https://nenkai.github.io/relink-modding/resources/item_ids/), [Model](https://nenkai.github.io/relink-modding/resources/model_ids/), [Phase](https://nenkai.github.io/relink-modding/resources/phase_ids/), [Quest](https://nenkai.github.io/relink-modding/resources/quest_ids/), [Sigil/Gem](https://nenkai.github.io/relink-modding/resources/sigil_gem_ids/), [Trait/Skill](https://nenkai.github.io/relink-modding/resources/trait_skill_ids/)
- 플레이어 시스템: [Action IDs](https://nenkai.github.io/relink-modding/resources/player/action_ids/), [Buff IDs](https://nenkai.github.io/relink-modding/resources/player/buff_ids/), [Control Types](https://nenkai.github.io/relink-modding/resources/player/control_types/), [Debuff/Ailment IDs](https://nenkai.github.io/relink-modding/resources/player/debuff_ailment_ids/), [Motions](https://nenkai.github.io/relink-modding/resources/player/motions/)
- 확률/보상: [Enemy Parts](https://nenkai.github.io/relink-modding/resources/enemy_break_part_rates/), [Transmute](https://nenkai.github.io/relink-modding/resources/gacha_rates/), [Curio](https://nenkai.github.io/relink-modding/resources/curio_loot_rates/), [Quest Drops](https://nenkai.github.io/relink-modding/resources/quest_drop_rates/), [ER Quest Drops](https://nenkai.github.io/relink-modding/resources/quest_drop_rates_er/), [Summon Traits](https://nenkai.github.io/relink-modding/resources/summon_trait_chances/), [Weapon Materials](https://nenkai.github.io/relink-modding/resources/weapon_materials/)

큰 CSV/표는 이 저장소로 복사하지 않는다. 필요할 때 원문 버전과 생성 근거를 확인하고, Djeeta MOD에 포함할 데이터만 재현 가능한 생성기로 좁혀 가져온다.

### 파일 형식과 게임 메커니즘

- 형식: [`.cfct`](https://nenkai.github.io/relink-modding/resources/formats/cfct/), [`.minfo`](https://nenkai.github.io/relink-modding/resources/formats/minfo/), [`.mmat`](https://nenkai.github.io/relink-modding/resources/formats/mmat/), [`.objread`](https://nenkai.github.io/relink-modding/resources/formats/objread/), [`.stpr`](https://nenkai.github.io/relink-modding/resources/formats/stpr/), [Effect callselector](https://nenkai.github.io/relink-modding/resources/formats/call_selector/)
- 메커니즘: [Quest Result Titles](https://nenkai.github.io/relink-modding/resources/re/mechanics/quest_evaluation/), [Quick Quest Power Scaling](https://nenkai.github.io/relink-modding/resources/re/mechanics/quick_quest_power_scaling/), [Overmasteries](https://nenkai.github.io/relink-modding/resources/re/mechanics/overmasteries/), [PWR](https://nenkai.github.io/relink-modding/resources/re/mechanics/pwr_power/), [Roll of the Die](https://nenkai.github.io/relink-modding/resources/re/mechanics/rotd/), [Sigil Synthesis](https://nenkai.github.io/relink-modding/resources/re/mechanics/gem_mix/), [Special Emotes](https://nenkai.github.io/relink-modding/resources/re/mechanics/emotes/), [Terminus Weapon Rolling](https://nenkai.github.io/relink-modding/resources/re/mechanics/terminus/)

## 권장 조사 절차

새 기능이나 게임 업데이트 대응 때 다음 순서를 권장한다.

1. 정확한 게임 버전과 실행 파일 SHA-256을 기록한다.
2. 사이트의 ER 호환성 페이지와 관련 자료의 최근 수정일을 확인한다.
3. 현재 릴리스의 `GBFRDataTools`와 파일 목록을 고정하고 해시를 기록한다.
4. 필요한 파일만 새 임시 디렉터리에 추출한다.
5. 테이블 변환에는 정확한 `-v 2.0.2`를 지정하고 도구의 현재 헤더를 사용한다.
6. 정적 파일에서 이름/ID/관계를 찾은 뒤, 메모리 구조는 live probe로 별도 검증한다.
7. 시그니처는 고정 실행 파일에서 유일성, 주변 명령, 호출 규약과 제어 흐름을 확인한다.
8. 자동 테스트와 오프라인/비공개 수동 smoke test를 모두 남긴다.

## 한계

- 사이트는 공동 역공학 문서라 `Unknown`, `TODO`, 추정 표현이 많다.
- 일부 표 헤더와 파일 경로는 미확인 또는 의도적으로 비공개다.
- 여러 페이지의 시그니처가 1.3.1 등 구버전을 명시하거나 버전을 명시하지 않는다.
- 사이트의 표·ID 목록은 최신 2.0.2 데이터 전체를 보장하지 않는다.
- 이 문서는 2026-07-24의 스냅샷을 기준으로 한다. 이후 변경은 [원본 저장소](https://github.com/nenkai/relink-modding) 이력에서 확인한다.
