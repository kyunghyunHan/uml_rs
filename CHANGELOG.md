# Changelog

## 0.1.0 — Unreleased

- 세션별 자동 복구와 기존/복수 사본 복원. 다른 창의 복구를 저장 과정에서 삭제하지 않음.
- 파일 revision 검사와 협력 저장 잠금으로 동시 편집 덮어쓰기 차단.
- 원자적 저장의 기존 권한 보존, Unix 부모 디렉터리 sync, 실패 원본 보존 테스트.
- 과도한 유한 route 좌표 거부, 포함 순환과 프래그먼트 참여자 참조 진단.
- 중첩 패키지/경계 이동·정렬·균등 배치 수정.
- 유스케이스 Auto의 직선 우선 및 액터 이름 회피 보존.
- 장면 캐시·변경 노드 reflow·viewport culling. Light/Dark 설정.
- 랜덤 편집/복구/충돌/실패 주입/캐시 회귀 테스트와 4가지 크기의 성능 harness.
- macOS 아이콘·ad-hoc signed .app·ZIP/해시, 3 OS CI와 draft Release workflow.
- 감사·아키텍처·성능·사용자·출시·라이선스 문서.

저장 형식은 v3, v1/v2 migration 유지. 기존 Use Case/Class/Sequence 기능 보존.
공개 Release Ready 판정 전: 프로젝트 라이선스, 의존성 원문 검토, Developer ID/공증, 실제 IME/파일 대화상자·다운로드 설치 검증이 남아 있다.
