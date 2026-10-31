# Release readiness 감사

2026-11-01 (Asia/Seoul). 시작 커밋 `effa076`, 시작 시 작업 트리 깨끗함. 원격 저장소를 변경하거나 push하지 않았다. 판정: **공개 릴리스 준비 미완료**. 코드 개선·자동 검증·로컬 패키징 결과와 사람/다른 환경의 검증이 필요한 항목을 구분한다.

## 조사 범위와 초기 상태

`Cargo.toml`/lock, `src` 20개 모듈, 기존 통합 테스트 5개, v1 fixture, 예제 프로젝트·내보내기·GUI 예제, 기존 UI/관계 검토 문서, NanumGothic TTF/OFL을 조사했다. 기존 테스트 79개 및 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`가 통과했다. CI, macOS 패키징 스크립트, 프로젝트 자체 LICENSE는 없었다. 기존 라우팅·탭·Inspector·한글 폰트·클립보드·Undo/Redo를 중복 구현하지 않았다.

## 실제 발견 및 적용 결과

### A01 · P0 · 인스턴스 사이 복구 데이터 손실

- 위치: 기존 `storage::config_dir`, `app::new/save/perform/update`의 공통 `recovery.umlproj`.
- 재현 조건: 두 창에서 서로 다른 미저장 프로젝트를 편집하고 자동 복구 시각을 경과시키거나, 한 창에서 저장/변경 버리기.
- 근거/영향: 두 창이 같은 경로에 덮어쓰고, 어느 창의 정상 저장도 공통 파일을 삭제했다. 다른 창의 유일한 디스크 복구 사본을 잃을 수 있었다. 프로세스 강제 종료를 통한 실데이터 파괴 실험은 하지 않았다.
- 수정: 세션별 `RecoveryStore`, 자기 사본만 정리, 기존/복수 복구 목록, 복구 원본 유지. 복구 삭제 오류를 UI에 전달.
- 결과/검증: `recovery_sessions_and_legacy_snapshots_remain_independent`, 앱 저장 테스트. 실제 두 OS 창의 동시 강제 종료는 미검증.

### A02 · P1 · 외부 수정 무조건 덮어쓰기

- 위치: 기존 `app::save` → `storage::save`.
- 재현 조건: 같은 파일을 두 창에서 연 뒤 차례로 다른 내용 저장.
- 근거/영향: 로드 시점의 원본 식별/비교 없이 atomic rename만 수행하여 먼저 저장한 변경을 잃는다. atomic write는 동시 편집 병합을 제공하지 않는다.
- 수정: 실제 로드 바이트의 `FileRevision`, 짧은 `.uml-lock`, 저장 직전 비교 및 다른 이름 저장 안내.
- 결과/검증: 두 독립 revision, 원본 삭제, 다른 writer의 잠금, 앱 경로를 통한 충돌/dirty/recovery 유지 테스트 통과. 잠금을 무시하는 외부 writer의 마지막 순간 변경과 경로 별칭 경쟁은 제한으로 남는다.

### A03 · P1 · 과도한 유한 waypoint 좌표

- 위치: `model::validate_structure`는 waypoint/bend의 finite만 검사했고 `scene::paint`의 점선 while은 길이에 비례했다.
- 재현 조건: JSON에 `1e30` 같은 유한 waypoint를 둔 dependency 관계.
- 근거/영향: 노드는 ±1,000,000 제한이 있었으나 route에는 없었다. 매우 긴 점선을 반복 그리는 동안 부동소수점 증가가 멈출 수도 있다. 네이티브 창을 의도적으로 무한 대기시키지는 않았다.
- 수정/검증: 노드와 동일한 route 좌표 제한, `extreme_finite_route_coordinates_are_rejected_before_rendering`으로 load/recovery 거부. 저장 형식은 v3 유지.

### A04 · P1 · 원자적 저장의 권한/디렉터리 동기화

- 위치: `storage::atomic_write`.
- 재현 조건: 권한을 제한한 기존 파일 저장, rename 직후 전원/OS 중단.
- 근거/영향: 기존 코드는 새 임시 파일의 기본 권한으로 교체했고 부모 디렉터리를 sync하지 않았다. 실제 전원 중단 내구성은 미검증.
- 수정: Unix 신규 임시 파일 0600, 기존 권한 유지, 파일 닫기 후 rename, Unix 부모 sync. 실패 시 임시 파일 정리.
- 결과/검증: 중간 write 오류 주입, rename 실패, 부모 없음, 0600 권한 보존, 기존 저장 왕복 통과. 실제 read-only 디렉터리에서 CLI 저장이 오류 코드 1로 종료하고 파일을 만들지 않음을 확인. 실제 ENOSPC·파일시스템별 crash durability는 별도 체크리스트에 남김.

### A05 · P1 · 중첩 포함 이동 누락 및 순환 미진단

- 위치: `commands::move_nodes/align/distribute`, `validation::project_diagnostics`.
- 재현 조건: 패키지 → 패키지 → 클래스에서 최상위 이동/정렬. 또는 파일의 boundary ID로 자기/상호 포함 생성.
- 영향: 직접 자식만 이동해 손자 좌표가 남고 의미 없는 포함 순환이 정상 저장 가능했다.
- 수정: 방문 집합 기반 전체 하위 이동, 가장 가까운 선택 조상의 배치 delta, 순환 진단. 기존 파일은 읽어서 수정/복구 가능.
- 검증: 중첩 이동·중복 선택·정렬·균등 배치·순환의 진단과 손실 없는 복구 테스트.

### A06 · P2 · 유스케이스 Auto의 불필요한 직교 경로

- 위치: `routing::path`의 UseCase 전용 `legacy_path` 조기 반환.
- 재현 조건: 장애물 없는 대각선 include/extend.
- 영향: Class의 자동 직선 우선 정책과 달리 불필요한 꺾임.
- 수정: 공통 직선 우선 검사. 액터 이름을 가로지르는 경우는 기존 우회 유지. 수동 waypoint/bend 우선순위 보존.
- 검증: 추가 자동 유스케이스 테스트 및 기존 액터 일반화·전체 라우팅/화살표/직교 회귀 테스트. 액터 일반화 기존 테스트가 초기 변경에서 실패하여 이름 회피를 유지하도록 보정했다.

### A07 · P2 · 매 프레임 전체 장면·텍스트 계산

- 위치: `canvas::ui`, `scene::build/paint`, `app::update`.
- 근거: 초기 Stress CPU 프레임 19.405ms, 확대 20.505ms. GPU 프레임 측정은 아니다.
- 수정: 현재 다이어그램 SceneCache, 바뀐 노드만 reflow, viewport culling.
- 결과: Stress 프레임 11.424ms, 확대 11.810ms. 드래그는 19.315 → 20.393ms로 개선 못 함. cold layout/route 시간도 개선 주장하지 않음.
- 검증: cache의 view 재사용/내용·Undo 무효화, culling 후 hit/bounds 유지, 전후 CSV. 상세는 performance-report.

### A08 · P2 · 잘못된 프래그먼트 참여자 범위 미진단

- 위치: `validation::project_diagnostics`, `sequence_layout`의 범위 fallback.
- 재현 조건: 존재하지 않는 `participant_start/end` ID가 있는 v3 파일.
- 영향: 설정한 범위 대신 전체/기본 범위로 그려져 잘못된 참조를 놓침.
- 수정/검증: 진단 추가, 손실 없이 로드·복구, 정상 저장 전 수정. 구조를 훼손하거나 파일에서 참조를 자동 삭제하지 않음.

### A09 · P2 · Light 전용 사용자 UI

- 위치: `theme`, custom `components`/`palette` painters, `Preferences`.
- 수정: 보기 메뉴의 Dark 토글 및 기본값 있는 설정 저장. custom 탭/팔레트 대비도 변경. 캔버스는 흰 종이, PNG/SVG는 테마 독립.
- 검증: headless dark 이미지 직접 확인, 설정 v1 호환/색 대비 테스트. 기존 작은 창·탭·입력·관계 생성 egui 이벤트 테스트 유지.

### A10 · P2 · CLI 예제 저장 panic

- 위치: `main`의 `.expect("example save")`.
- 수정/검증: 오류 메시지와 종료 코드 1. 파일시스템 저장 실패는 panic 없이 반환. 배포 체크의 CLI 왕복/내보내기로 실행 검증.

### A11 · P1 · 배포 구성과 라이선스 누락

- 위치: 기존 루트에 CI/.app/CHANGELOG/LICENSE 없음.
- 수정: 3 OS CI, macOS 아이콘/Info.plist/내장 리소스/ad-hoc 서명/ZIP/해시, tag 기반 draft release 워크플로, 사용자·출시 문서, 의존성 메타데이터와 원문 수집.
- 결과: 로컬 arm64 `.app`/ZIP 생성과 서명 검증. 프로젝트 라이선스 미지정 및 원문 자동 수집 누락 32개는 해결되지 않음. 배포 소유자의 결정/검토 없이 공개 Ready로 판정하지 않음.

## UML 및 입력 기능 감사 매트릭스

| 영역 | 확인한 기능·근거 | 한계/추가 검증 |
|---|---|---|
| Use Case | 액터 selectable box, 한글 줄바꿈, include/extend/일반화 방향, 명세와 sequence ID, 명시적 boundary, 겹침 선택. connections/improvements/workflows | 실제 트랙패드/IME는 수동 |
| Class | Class/abstract/interface/enum/note/package, association·탐색 방향·aggregation·composition·generalization·realization·dependency·note link. relationship_editing와 connections | 전용 enum literal 및 모든 UML 패키지 의미 모델 없음 |
| Class 배치 | 테두리 포트, move/resize, 다중성/역할, static 밑줄/abstract 기울임, 자동/수동 크기와 overflow 안내 | 수동 높이 부족 시 텍스트 overflow 유지 및 경고, 자동 축소 없음 |
| Sequence | sync/async/return/self/create/destroy, 이벤트·참여자 순서, 생명주기, 중첩 활성/alt/opt/loop, 타입/메서드 ID | 캔버스 메시지 드래그 재정렬 없음, 목록에서 가능. alt 2분기·부분 교차 범위 제한 |
| Route | 직선/직교/자동, 장애물·포트·루프·수동점·swap·화살촉·label 회피와 PNG/SVG 일치 | 최대 격자 초과 시 best-score fallback. 밀집 그래프의 무교차/라벨 최적성·작은 이동의 전역 안정성 보장 없음 |
| 이력 | 100 snapshot, 연속 입력/drag transaction, view 보존, copy/paste ID 재발급 | 대규모 프로젝트 이력 메모리·전체 프로젝트 clone 비용 남음 |
| 저장 | v1/v2→v3, Unicode 파일명, 손상/중복ID/참조 진단, 32MiB 제한, 복구 | 실제 power loss/ENOSPC/네트워크 FS는 미검증 |
| UI | 기존 상단 작업·좌측 트리·탭·캔버스·Inspector·진단, 820×620 패널 접기, HiDPI native 캡처 | 전면 재디자인 대신 기존 구조 유지. 접근성 리더·OS별 대화상자 미검증 |

지원 요청에 나열된 관계는 이미 모델과 UI가 지원하므로 새 UML 관계 enum을 추가하지 않았다. Directed association은 별도 의미 kind를 만들지 않고 association의 양 끝 탐색 상태로 표시한다.

## 검증/출시 판정

최종 명령별 결과와 네이티브/패키지 증거는 [release-checklist](release-checklist.md), 실제 수치는 [performance-report](performance-report.md)에 기록한다. Windows/Linux는 CI를 구성했지만 이 로컬 작업에서 실행하지 않았다. GitHub Actions와 Releases도 로컬 생성만 했으며 원격 실행·게시하지 않았다. 공식 runner 표의 macos-14 arm64는 [GitHub 문서](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)로 확인했다.

다음 릴리스는 저장 crash/다중 프로세스 시나리오, 실제 입력 장치/IME/파일 대화상자, Developer ID·공증·격리 다운로드 설치, 라이선스 결정을 먼저 완료한 뒤 밀집 그래프 profiling·증분 diagnostics/command history·접근성을 진행한다.


## 수정/추가 파일

- Rust 핵심: `src/storage.rs`, `src/app.rs`, `src/model.rs`, `src/validation.rs`, `src/commands.rs`, `src/routing.rs`, `src/scene.rs`, `src/canvas.rs`, `src/theme.rs`, `src/components.rs`, `src/palette.rs`, `src/properties.rs`, `src/main.rs`.
- 자동 검증/예제: `tests/release_safety.rs`, storage/app 단위 테스트, `examples/performance.rs`, `examples/app_icon.rs`, `examples/gui_smoke.rs`, `examples/ui_snapshot.rs`.
- 빌드/배포: `Cargo.toml` 메타데이터, `.gitignore`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `scripts/package-macos.sh`, `scripts/collect-licenses.py`, `scripts/smoke-installed-macos.py`, `assets/icon.svg`.
- 문서: `README.md`, `CHANGELOG.md`, `docs/audit-report.md`, `docs/architecture.md`, `docs/performance-report.md`, `docs/release-checklist.md`, `docs/user-guide.md`, `docs/release-notes.md`, `docs/third-party-notices.md`, `docs/dependency-licenses.csv`, `docs/release-evidence/`.
- 로컬 배포 산출물(ignored): `dist/UML Studio.app`, `dist/UML-Studio-0.1.0-macos-arm64.zip`, SHA-256, 아이콘 세트, 의존성 원문 수집 디렉터리. 공개 업로드하지 않았다.

최종 자동 결과: fmt/check-all-targets/Clippy(-D warnings)/test-all-targets/release build 모두 통과. 최종 테스트 96개(기존 79 + 추가 17). 네이티브 3종 화면 및 headless 작은 창/Dark 확인, ZIP 해제본 서명·CLI 한글 경로 save·3종 PNG/SVG, bundle executable 5초 시작 검증 통과. 패키지 실행 프로세스는 미편집 상태에서 종료했다. 상세 로그는 `release-evidence`에 있다.
