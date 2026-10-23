# 클래스 관계 편집·경로 검증

기존 일반 연관의 무화살표 실선은 올바른 동작이었으며 유지했다. 도착점 화살표는 기존 `navigable=true`일 때 이미 그려지고 있었다. 개선이 필요했던 부분은 도착점 하나만 표현하던 bool 모델, 생성 메뉴와 속성의 서로 다른 허용 목록, 직교 후보에 치우친 자동 경로와 부족한 라벨 충돌 회피였다.

**변경 파일과 동작**

| 파일 | 변경 |
| --- | --- |
| `src/model.rs` | 양 끝 `Navigability`, 방향 프리셋, 경로 모드, 연결점의 테두리/비율, 끝 교환. 다이어그램별 허용 종류를 한 목록으로 제공 |
| `src/migration.rs`, `src/storage.rs` | v1/v2 읽기 → v3. `false`는 양 끝 미지정, `true`는 시작 미지정·도착 탐색 가능. 기존 ID와 수동 경로 보존 |
| `src/validation.rs` | 클래스 시작점 조기 안내, 반대편을 고려한 끝 후보 검사, 다중성 구문·범위, 합성 전체 상한 검사. 생성·수정·진단·정상 저장의 기존 공통 검증 유지 |
| `src/palette.rs`, `src/app.rs` | 일반/방향 연관 네 프리셋과 모든 관계의 선·화살촉 미리보기. 속성과 허용 목록 공유. 노드 중앙선 정렬을 더보기에서 제공 |
| `src/properties.rs` | 끝별 미지정/탐색 가능/x, 방향 프리셋, 시작↔도착 교환, 후보별 금지 이유, 역할·다중성 프리셋, 모드·연결점·경로 초기화 |
| `src/commands.rs` | 수동 꺾임을 포함한 경로를 반전하고 양 끝 역할·다중성·탐색 상태·연결점을 함께 교환 |
| `src/routing.rs` | 직선·직교·자동, 공통 높이 연결, 중복/같은 방향 점 제거, 장애물 후보·가시성 격자, 자기 루프, 겹친 자동 관계 간격, 포트 정렬·스냅, 라벨 위치 |
| `src/canvas.rs` | 연결점 드래그/가이드, 관계 우클릭 모드·초기화, 선택 요소/마지막 선택 영역 맞춤 |
| `src/scene.rs` | 메뉴·캔버스·SVG·PNG가 공유하는 열린 화살표/x/다이아몬드. 이름·역할·다중성이 노드·끝 장식·선·이전 라벨을 피하도록 배치 |
| `src/demo.rs`, `examples/relationship_review.rs` | 방향, x, 다이아몬드, 자기 연결, 중복 관계와 장애물을 포함한 검토 프로젝트·PNG/SVG 생성 |
| `examples/ui_snapshot.rs` | 관계 속성·경로 설정·생성 메뉴의 실제 egui 화면 재생성 옵션 |
| `tests/relationship_editing.rs`, `tests/connections.rs`, `tests/improvements.rs`, `src/app.rs` 테스트 | 모델·파일·실제 입력·렌더링 검증. 기존 핸들 테스트는 새 2점 직선의 중간 핸들도 검사 |
| `README.md`, `docs/screenshots/{class,usecase}-headless.png` | 사용법·버전 안내와 화면 갱신 |

다이아몬드는 전체(시작) 쪽에 유지되며 양 끝 탐색 상태와 독립적이다. 시작 쪽에 다이아몬드와 탐색 화살표를 함께 표시할 때는 겹치지 않도록 다이아몬드 바깥 꼭짓점에 탐색 화살표를 잇는다. 유스케이스의 기본 연관 표기와 기존 static 밑줄, abstract 기울임, 인터페이스 표기는 유지했다.

**실행한 검증**

- `cargo test --all-targets`: 79개 통과. 생성 메뉴의 네 방향 → 실제 연결 → 속성 프리셋/명시적 탐색 불가 → 끝 교환 → Undo/Redo를 포함한다.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check`: 통과.
- 1440×900 및 820×620에서 속성의 경로 모드·연결점 선택이 즉시 모델/경로에 반영되는 입력 테스트 통과.
- 우클릭 경로 모드/초기화, 연결점 드래그와 중앙/다른 끝의 X/Y 스냅, 선택 맞춤, 수동 경로·메타데이터 반전, v1/v2 마이그레이션과 v3 왕복 저장 검사 통과.
- 수평·수직·대각선·겹친 노드·장애물·수동 경로·양방향·자기 연결·중복 관계 검사 통과. 자기 연결은 모든 테두리 조합에서도 노드를 가로지르지 않는 루프를 검사했다.
- 0.5×/1×/1.8× 캔버스 렌더링의 화살표·삼각형·다이아몬드 좌표와 SVG 도형 좌표를 검사했다. PNG와 내보낸 SVG의 재렌더링도 픽셀 비교했다. 마지막 선분과 화살촉 방향, 라벨/노드 충돌, 기존 글자 표기도 별도 검사했다.
- 아래 이미지와 클래스·유스케이스 기본 화면을 최종 코드로 새로 생성하여 시각적으로 확인했다. 검토 이미지에서 발견한 자기 루프 이름/장애물 우회선 겹침을 수정했다.

| 결과물 | 링크 |
| --- | --- |
| 관계 표기 전체 PNG / SVG / v3 프로젝트 | [PNG](relationships.png) · [SVG](relationships.svg) · [프로젝트](relationships.umlproj) |
| 1440×900 관계 속성 | [화면](editor-wide.png) |
| 820×620 관계 속성 | [화면](editor-small.png) |
| 1440×900 생성 메뉴와 관계 미리보기 | [화면](menu-wide.png) |
| 1440×900 경로·연결점 속성 | [화면](routes-wide.png) |
| 820×620 경로·연결점 속성 | [화면](routes-small.png) |

**남아 있는 제약과 미실행 항목**

- 직선 모드는 장애물을 우회하지 않는다. 명시적으로 고른 연결점이나 수동 경로가 노드 내부를 통과하면 속성에서 안내하며, 노드 배치를 임의로 바꾸지 않는다.
- 수동 `waypoints`/`bend`가 있으면 모드를 바꿔도 그 경로를 우선 보존한다. 자동 계산으로 돌아가려면 **자동 경로 초기화**를 사용한다. 연결점 선택은 별도 설정으로 유지한다.
- 구현을 인터페이스→클래스로 뒤집거나, 합성 교환 후 전체 다중성이 1을 초과하는 수정은 UML 검증이 거부한다. 오류와 수정 초안에서 바로잡아야 적용된다.
- 가시성 격자는 40,000점으로 제한하고, 초과 시 가장 나은 후보 경로를 사용한다. 겹친 노드·극도로 밀집한 그래프의 교차/라벨 충돌을 모두 해소한다고 보장하지 않는다. 명시적 연결점과 수동 경로는 자동 평행선 분리보다 우선한다.
- 새 저장은 v3이며 구버전 앱에서 읽는 하위 호환은 제공하지 않는다.
- 편집기 화면은 headless egui의 실제 메시·글자를 CPU로 렌더링했다. 네이티브 OS 창/GPU와 파일 대화상자는 이번 작업에서 실행 검증하지 않았다.

재생성:

```sh
cargo run --example relationship_review -- docs/relationship-review
cargo run --example ui_snapshot -- docs/relationship-review/editor-wide.png 1 --relation-inspector
cargo run --example ui_snapshot -- docs/relationship-review/editor-small.png 1 --small --inspector --relation-inspector
cargo run --example ui_snapshot -- docs/relationship-review/menu-wide.png 1 --relation-menu
cargo run --example ui_snapshot -- docs/relationship-review/routes-wide.png 1 --relation-inspector --route-settings
cargo run --example ui_snapshot -- docs/relationship-review/routes-small.png 1 --small --inspector --relation-inspector --route-settings
```
