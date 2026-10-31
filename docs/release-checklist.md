# 출시 체크리스트

2026-11-01 / 0.1.0 Unreleased / macOS Apple Silicon 우선. **현재 공개 Release Ready 아님.**

## 자동 및 로컬 검증

| 항목 | 결과 | 근거 |
|---|---|---|
| cargo fmt --check | 통과 | 최종 로그 release-evidence |
| cargo check --all-targets | 통과 | 모든 예제/테스트 포함 |
| cargo clippy --all-targets -- -D warnings | 통과 | 경고 0 |
| cargo test --all-targets | 통과, 96개 | 초기 79개 → 최종 96개 |
| cargo build --release | 통과 | arm64 Mach-O, minos 11.0 |
| UML 기존 기능 | 자동 검증 통과 | 관계/화살표/히트/라우트/Undo/참조/시퀀스/PNG/SVG |
| v1/v2 migration 및 v3 round-trip | 통과 | 기존 fixture와 v1/v2 navigation 테스트 |
| 저장/복구 실패 | 자동 검증 통과 | 중간 write 주입, rename/부모 없음, 원본 보존/권한/충돌/잠금/세션 격리. 실제 read-only 폴더 CLI 오류 코드 1도 확인 |
| 한글/Unicode | 자동 검증 통과 | 파일명·내용·glyph·SVG outlined·텍스트 입력 |
| 네이티브 GUI | macOS 실행·캡처 통과 | native-usecase / native-sequence-small / native-class-dark |
| 작은 창 / HiDPI | 자동 입력/배치 및 native 캡처 | headless 820×620, native Retina는 실제 디스플레이 가용 높이에 맞춰 조정될 수 있음 |
| Light/Dark | 코드/설정 테스트 및 이미지 확인 | headless-dark.png, native-class-dark.png |
| .app/아이콘/내장 폰트 | 로컬 생성 통과 | package-macos.sh, iconutil, Info.plist lint |
| ad-hoc signature | 통과 | codesign --verify --deep --strict |
| ZIP 해제 후 로컬 실행/CLI | 통과 | 서명·한글 경로 save·3종 PNG/SVG, bundle 실행 5초 유지. installed-smoke.log |
| 성능 | 전후 4규모 CSV | Stress idle/zoom 개선, drag 16.7ms 미달 |
| CI / draft release 구성 | 파일 작성 및 로컬 명령 통과 | GitHub 원격 실행은 미실행 |

로그와 이미지는 `docs/release-evidence/`에 있다. `scripts/smoke-installed-macos.py`로 ZIP을 임시 폴더에 풀고 bundle 실행 파일을 직접 시작했다. Finder/LaunchServices 경로와 인터넷 격리 속성을 검증한 것은 아니다. 저장 기능의 파일시스템 검사는 임시 디렉터리에서 실행했다. native 창 테스트에는 실제 egui GPU renderer를 사용했으며 파일 대화상자를 자동 조작하거나 실제 키보드 IME 입력을 보내지는 않았다.

## 공개 출시 차단 항목

- [ ] 소유자가 프로젝트 LICENSE 및 Cargo license/license-file을 결정한다. 현재 미지정.
- [ ] macOS 대상 의존성 212개 인벤토리 중 로컬 패키지에 라이선스 원문이 없는 32개를 추가 확인한다. `dist/ThirdPartyLicenses/needs-review.txt` 참조. Windows/Linux 별도 목록도 필요.
- [ ] Apple Developer Program 계정, Team ID, private key가 있는 Developer ID Application 인증서 및 Keychain/공증 자격 증명을 준비한다. 이 작업에서 계정/인증서를 조회하거나 등록하지 않았다.
- [ ] Developer ID 서명·hardened runtime·공증·staple 검증을 완료한다. 로컬 ad-hoc 검증과 구분한다.
- [ ] Gatekeeper 기본 상태의 깨끗한 Mac에서 실제 다운로드 ZIP의 격리 상태·해제·Applications 복사·첫 실행·재실행을 확인한다.
- [ ] OS 파일 대화상자로 한글/공백/긴 경로 저장·열기·덮어쓰기·취소·접근 거부를 확인한다.
- [ ] 실제 한국어 IME 조합/후보 선택/취소 중 단축키·Undo/Redo·초점 전환을 확인한다.
- [ ] 실제 마우스·트랙패드 스크롤·핀치·drag·모니터 DPI 변경·800px급 창을 확인한다.
- [ ] 실제 두 인스턴스 편집과 강제 종료·복구·남은 잠금 처리, 저장 중 강제 종료/실제 ENOSPC/권한 오류를 확인한다. 오류 주입 테스트를 실제 전원 중단 실험으로 간주하지 않는다.
- [ ] Windows/Linux CI를 원격 실행하고 지원 대상으로 선언하기 전에 해당 OS GUI/파일 시스템 테스트를 수행한다. Rust 1.88 MSRV 실행도 아직 하지 않았다.

## 수동 서명/공증 절차 (미실행)

Apple의 [공증 안내](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)와 [custom workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)를 기준으로 Developer ID 인증서로 서명하고 `notarytool`로 제출한 뒤 ticket을 붙인다. 아래 명령은 실제 자격 증명이 준비된 배포 담당자가 실행한다. 공증은 계정과 네트워크 접근이 필요하다.

```sh
MACOS_SIGN_IDENTITY='Developer ID Application: OWNER (TEAMID)' bash scripts/package-macos.sh
# Keychain에 미리 준비한 profile을 사용한다. 비밀값을 저장소에 쓰지 않는다.
xcrun notarytool submit dist/UML-Studio-0.1.0-macos-arm64.zip --keychain-profile UML_STUDIO_NOTARY --wait
xcrun stapler staple 'dist/UML Studio.app'
xcrun stapler validate 'dist/UML Studio.app'
codesign --verify --deep --strict --verbose=2 'dist/UML Studio.app'
spctl --assess --type execute --verbose=4 'dist/UML Studio.app'
# Staple 후 최종 아카이브와 해시를 다시 만든다. package script를 재실행하면 재서명되므로 피한다.
ditto -c -k --keepParent 'dist/UML Studio.app' dist/UML-Studio-0.1.0-macos-arm64.zip
(cd dist && shasum -a 256 UML-Studio-0.1.0-macos-arm64.zip > UML-Studio-0.1.0-macos-arm64.zip.sha256)
```

CI의 `macos-14`는 [GitHub의 runner 표](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)에 따라 arm64로 선택했다. package script는 실제 `uname -m`을 파일명에 반영한다. 최소 배포 타깃 11.0은 Mach-O/Info.plist 값이며 macOS 11 실기기에서 검증했다는 뜻은 아니다.

## 다음 릴리스 우선순위

1. 위 데이터 안전·IME·설치·라이선스·공증 게이트를 완료한다.
2. Stress drag/밀집 라우팅 p95와 100개 Undo 상태 메모리를 계측한다. 필요하면 진단·참조 인덱스·증분 라우팅·명령 이력을 도입한다.
3. 캔버스 시퀀스 메시지 재정렬, 다분기 alt, enum literal 편집 및 접근성은 별도 호환성 설계 후 진행한다.

GitHub tag를 push하면 기본 워크플로는 **draft**만 생성한다. 공개 게시 전 체크리스트를 검토해야 하며 이번 작업에서는 tag/push/release publish를 하지 않았다.
