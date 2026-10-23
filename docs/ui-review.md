# UI 정리 및 검증

2026-10-23 기준. 실제 egui 코드에서 화면을 생성했다.

- `src/theme.rs`: 색상, 글자 크기, 32px 컨트롤, 26px 아이콘 영역, 8px 간격, 12px 패널 여백과 프레임.
- `src/components.rs`: 다이어그램 탭, 종류 아이콘, 목록 행, 입력창, 필드 라벨, 삭제 버튼과 고정 높이 안내.
- `src/app.rs`: 전역 메뉴 / 탭 / 편집 도구 / 보기 옵션 분리, 패널 정렬과 빈 화면에서 다시 열기. 탭 닫기는 문서를 삭제하지 않는다.
- `src/palette.rs`: 공통 선택 색상, 시퀀스 도구의 명시적 아이콘·텍스트 배치.
- `src/properties.rs`: 입력 너비 통일, 필드 라벨과 관계 그룹, 좁은 패널의 줄 배치, 하단 삭제 버튼.
- `src/canvas.rs`: 낮은 대비의 성긴 점 그리드, 파란 선택·연결 시작점 표시, 캔버스 오류 안내 유지.
- `src/lib.rs`: 새 모듈 등록.
- `tests/editor_components.rs`, `src/app.rs` 테스트: 탭 클릭/닫기/재열기, 키보드 닫기, 말줄임과 고정된 닫기 영역, 도구 영역 높이, 속성 편집과 이력.
- `examples/ui_snapshot.rs`: 모든 탭 표시, `--inspector`, `--long-names` 검토 옵션.

`cargo test --all-targets`의 63개 테스트가 통과했다. 관계 메뉴에서 include/extend/일반화를 선택하여 생성하고 Undo/Redo하는 기존 통합 검사와 잘못된 연결 안내 검사도 통과했다. UML 연결 허용 규칙은 변경하지 않았다.

1440×900 및 820×620에서 탭 선택, 비활성 탭 닫기 시 선택 유지, 마지막 탭 닫기와 다시 열기, 속성 이름 입력 및 Undo/Redo, 도구 변경 시 캔버스 크기 유지, 주요 버튼의 배치와 줄바꿈을 egui 입력 이벤트로 검사했다. 속성 패널은 긴 다이어그램 이름과 과도한 저장 너비를 복원한 경우에도 폭 제한을 통과했다.

아래 7개 PNG를 최종 코드로 다시 생성하여 확인했다. 버튼·입력창 겹침이나 의도하지 않은 툴바 줄바꿈은 보이지 않았다. 좁은 화면의 넘치는 탭은 기존 가로 스크롤 영역 안에 표시된다.

| 화면 | 이미지 |
| --- | --- |
| 1440×900 유스케이스 | [보기](screenshots/usecase-headless.png) |
| 1440×900 클래스 | [보기](screenshots/class-headless.png) |
| 1440×900 시퀀스 | [보기](screenshots/sequence-headless.png) |
| 1440×900 긴 탭 이름 | [보기](screenshots/tabs-long-headless.png) |
| 820×620 기본 | [보기](screenshots/small-headless.png) |
| 820×620 클래스 속성·긴 탭 이름 | [보기](screenshots/small-inspector-headless.png) |
| 820×620 시퀀스 속성 | [보기](screenshots/small-sequence-headless.png) |

재생성 예시:

```sh
cargo run --example ui_snapshot -- docs/screenshots/usecase-headless.png 0
cargo run --example ui_snapshot -- docs/screenshots/class-headless.png 1
cargo run --example ui_snapshot -- docs/screenshots/sequence-headless.png 2
cargo run --example ui_snapshot -- docs/screenshots/tabs-long-headless.png 0 --long-names
cargo run --example ui_snapshot -- docs/screenshots/small-headless.png 0 --small
cargo run --example ui_snapshot -- docs/screenshots/small-inspector-headless.png 1 --small --inspector --long-names
cargo run --example ui_snapshot -- docs/screenshots/small-sequence-headless.png 2 --small --inspector
```

이미지는 headless egui의 실제 도형·폰트·메시를 CPU로 렌더링한 결과다. 이번 변경에서는 네이티브 OS 창과 GPU 출력, 파일 대화상자는 실행 검증하지 않았다.
