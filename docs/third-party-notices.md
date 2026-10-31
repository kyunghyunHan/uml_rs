# Third-party notices / 배포 라이선스 점검

2026-11-01. `Cargo.lock`과 macOS Apple Silicon 대상 `cargo metadata --locked --offline --filter-platform aarch64-apple-darwin`에서 직접 추출한 인벤토리는 [dependency-licenses.csv](dependency-licenses.csv)다. 개발 의존성도 포함하는 보수적인 목록이며 다른 OS 전용 의존성은 포함하지 않는다. SPDX 표현을 변경하거나 단일 라이선스로 해석하지 않았다.

- 포함 폰트: NanumGothic, `assets/NanumGothic-OFL.txt` 원문 포함. 폰트는 바이너리에 `include_bytes!`로 내장된다.
- `scripts/collect-licenses.py`는 로컬 Cargo 소스의 LICENSE/COPYING/NOTICE 원문을 복사한다. 결과의 `inventory.csv`와 `needs-review.txt`를 확인해야 한다. 패키지에 원문이 없으면 SPDX 메타데이터만으로 검토 완료로 간주하지 않는다.
- `.app/Contents/Resources/ThirdPartyLicenses`에 수집한 원문과 검토 목록을 넣는다.
- 저장소 자체의 LICENSE 및 Cargo `license`/`license-file` 필드는 없다. 소유자의 배포 라이선스 선택을 대신하지 않았다. **공개 배포 전 해결해야 하는 차단 항목**이다.
- 이번 목록은 취약점 감사나 법률 검토 결과가 아니다. Cargo audit/deny 및 모든 플랫폼의 라이선스 원문 검토는 미실행이다.

재생성:

```sh
cargo metadata --locked --offline --filter-platform aarch64-apple-darwin --format-version 1
python3 scripts/collect-licenses.py dist/ThirdPartyLicenses
```
