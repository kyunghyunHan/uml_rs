# 성능 측정

2026-11-01. Apple M1 8 cores / 16GiB, macOS 26.6.2 arm64, rustc 1.99.0, `--release`. 기본 의존성 및 format_version 3 유지. 결과 원문: [before CSV](release-evidence/performance-before.csv), [after CSV](release-evidence/performance-after.csv).

## 방법

`examples/performance.rs`가 20/20, 100/150, 500/750, 1,000/1,500개 노드/엣지의 한글 클래스 grid를 생성하고 저장한다. 노드 간격은 205×155, 연결은 인접 가로/세로 auto association이며 일부 평행선이 있다. UUID는 실행마다 다르지만 배치·연결 순서는 동일하다. 예제에 수동 waypoint·중첩 시퀀스·장애물이 밀집한 최악 경로를 넣지는 않았다.

- load/save: 실제 파일, 정상 검증·serialize·sync 포함. 각 1회. 파일시스템 캐시/SSD 지연 영향을 받는다.
- layout: `scene::build` 1회, 폰트 reflow·라우팅 포함. 첫 Small은 폰트 초기화도 포함한다.
- frame/drag/zoom: 실제 `UmlApp::update`와 egui tessellation을 창 없이 1440×900에서 수행. 3프레임 warmup 뒤 각각 5프레임 평균. drag는 노드 x를 +1, zoom은 +0.05로 바꾼 시뮬레이션이다. 실제 OS 마우스/트랙패드 이벤트나 GPU present 시간은 아니다. 기본 zoom에서 일부 영역만 보인다.
- routes: 모든 관계의 경로 재계산 1회. hit: 월드 좌표 100지점 평균.
- Undo/Redo: 한 이동의 undo+redo. 최종 harness는 black_box와 1,000회 평균으로 극히 짧은 비용을 측정한다. 초기 harness는 1회/소수점 3자리여서 0.000ms를 표시했으며 0이라는 의미가 아니다.
- PNG/SVG: 전체 장면, 1x/32px 여백/흰 배경, PNG 인코딩/outlined SVG·원자적 쓰기 포함. SVG는 텍스트를 path로 변환한다.
- RSS: 각 크기 export 후 `ps -o rss=`로 프로세스 현재 RSS(KiB)를 조회했다. 동일 프로세스가 Small→Stress 순서로 실행된다. 메모리 할당자의 잔류 및 export 이미지 메모리를 포함하며 프로젝트 단독 크기·peak·leak 측정은 아니다.

개선 전 release 실행파일을 보존하고 최종 실행파일과 빌드가 없는 같은 환경에서 연속 실행했다. CSV는 한 쌍의 관측치다. 통계적 유의성/타 OS 성능/밀집 그래프의 최악 지연은 주장하지 않는다.

## 결과 (ms)

| 크기 nodes/edges | load 전→후 | layout 전→후 | frame 전→후 | drag 전→후 | zoom 전→후 |
|---|---:|---:|---:|---:|---:|
| Small 20/20 | .674→.280 | .298→.063 | .304→.237 | .306→.273 | .373→.294 |
| Medium 100/150 | 1.048→.997 | .280→.293 | 1.033→.676 | 1.415→.930 | 1.080→.704 |
| Large 500/750 | 3.958→3.935 | 2.026→2.391 | 6.081→3.827 | 6.108→6.164 | 6.565→3.858 |
| Stress 1000/1500 | 8.528→8.757 | 8.322→9.606 | 19.405→11.424 | 19.315→20.393 | 20.505→11.810 |

| 크기 | routes 전→후 | hit 전→후 | save 전→후 | PNG 전→후 | SVG 전→후 | RSS KiB 전→후 |
|---|---:|---:|---:|---:|---:|---:|
| Small | .006→.006 | .006→.005 | 15.030→9.250 | 24.179→13.383 | 10.963→9.592 | 25888→26112 |
| Medium | .076→.095 | .025→.025 | 8.597→8.372 | 33.964→30.949 | 18.903→17.776 | 57168→56640 |
| Large | 1.123→1.564 | .121→.119 | 10.765→9.167 | 128.395→127.711 | 66.016→67.027 | 208000→206192 |
| Stress | 6.295→7.374 | .239→.238 | 12.366→13.575 | 244.127→240.349 | 124.479→125.831 | 498112→496624 |

최종 Undo+Redo 평균은 Small .000015 / Medium .000016 / Large .000013 / Stress .000012ms. 스냅샷 벡터의 swap이므로 작은 비용이며 **히스토리 record/프로젝트 clone 비용을 제외**한다. 이를 편집 전체 비용으로 해석하면 안 된다.

## 적용과 미달 사항

캔버스는 내용이 같으면 SceneCache를 재사용한다. pan/zoom은 재라우팅하지 않는다. 노드 reflow는 변경된 요소만 계산한다. paint에서 화면 밖 primitive를 제출하지 않는다. 캐시는 도메인·파일 형식과 분리했고 선택/히트 테스트/전체 출력은 보존했다.

이 관측에서 Stress idle CPU frame은 약 41%, zoom은 약 42% 감소했다. 60Hz의 16.7ms 예산은 해당 시뮬레이션의 idle/zoom에서 달성했지만 **Stress drag는 20.393ms로 미달**이다. 이동은 장면/경로를 다시 만들어야 하고 앱의 프로젝트 clone·진단·Catalog·선형 참조 탐색이 남아 있다. cold layout/routes도 향상되지 않았고 오히려 일부 수치가 증가했다. 따라서 라우팅 알고리즘이나 저장이 빨라졌다고 주장하지 않는다. 원자적 저장에는 추가 권한/디렉터리 sync/충돌 검사가 있다.

다음 측정은 타이머 span 또는 sampling profiler로 clone/validation/reflow/route/label을 분리하고, 밀집 그래프·수동 경로·시퀀스 및 p95/p99·실제 입력·GPU present·이력 100개 상태 RSS를 포함해야 한다. 현재 데이터로 spatial index/백그라운드 작업을 일괄 도입하지 않았다.

재현:

```sh
UML_STUDIO_HOME=/tmp/uml-benchmark cargo run --release --locked --example performance -- /tmp/uml-perf
```
