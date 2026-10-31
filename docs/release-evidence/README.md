# 실행 증거

2026-11-01, macOS Apple Silicon. 파일은 실제 도구 출력이며 시간/주소/스레드 ID는 실행마다 다르다.

- `fmt.log`: 정상 완료 시 빈 출력. `check.log`, `clippy.log`, `tests.log`(96개), `build.log`: 최종 필수 명령.
- `package.log`: iconset, plist, ad-hoc signature, ZIP 생성.
- `installed-smoke.log`: ZIP 해제본 서명, 한글 파일명 CLI 저장, PNG/SVG 각 3종, 실제 GUI 프로세스 5초 시작.
- `native-usecase.png`, `native-sequence-small.png`, `native-class-dark.png`: 실제 macOS eframe GPU viewport screenshot. 시스템 전체 화면/다른 앱은 캡처하지 않았다.
- `headless-dark.png`, `headless-small.png`: CPU egui mesh 렌더링. native 증거와 구분한다.
- `performance-before.csv`, `performance-after.csv`: 같은 기기의 release 실행 관측. 방법/제한은 `../performance-report.md`.

실제 마우스/IME/OS 대화상자/Gatekeeper 다운로드 설치/공증/Windows·Linux 실행/원격 CI는 이 증거에 포함하지 않는다.
