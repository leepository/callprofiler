# callprofiler 사용 가이드

Python 함수의 호출 흐름을 추적하고, 인터랙티브 HTML 콜 그래프로 시각화하는 프로파일링 라이브러리입니다.
Rust(PyO3) 기반으로 이벤트 파싱과 리포트 생성을 처리하여 오버헤드를 최소화합니다.

## 설치

### wheel 파일로 설치 (빌드 완료된 바이너리)

```bash
pip install target/wheels/callprofiler-0.1.0-cp312-cp312-macosx_11_0_arm64.whl
```

### 소스에서 빌드하여 설치

Rust 툴체인과 [maturin](https://github.com/PyO3/maturin)이 필요합니다.

```bash
# maturin 설치
pip install maturin

# 개발 모드 설치 (코드 수정 시 재빌드 가능)
maturin develop

# 릴리스 빌드
maturin build --release
pip install target/wheels/callprofiler-*.whl
```

### 요구사항

- Python >= 3.9
- Rust 툴체인 (소스 빌드 시)
- maturin >= 1.11 (소스 빌드 시)

## 기본 사용법

### 동기 함수 프로파일링

```python
from callprofiler import profile

@profile
def process_data():
    data = load_from_db()
    result = transform(data)
    save_result(result)
    return result

process_data()
# [callprofiler] Report saved to .profile/process_data_20260211_120000.html
```

### 비동기 함수 프로파일링

`async def` 함수에도 동일한 `@profile` 데코레이터를 그대로 사용합니다.
내부에서 자동으로 비동기 함수를 감지하여 `await` 실행 중의 호출 흐름까지 추적합니다.

```python
import asyncio
from callprofiler import profile

@profile
async def fetch_and_process():
    data = await fetch_from_api()
    result = await process_async(data)
    return result

asyncio.run(fetch_and_process())
# [callprofiler] Report saved to .profile/fetch_and_process_20260211_120000.html
```

### 출력 디렉토리 지정

기본 출력 경로는 `.profile/`이며, `output_dir` 파라미터로 변경할 수 있습니다.

```python
@profile(output_dir="reports")
def my_function():
    ...

@profile(output_dir="reports")
async def my_async_function():
    ...
```

## HTML 리포트 구성

생성된 HTML 파일을 브라우저에서 열면 다음 정보를 확인할 수 있습니다.

### 요약 (Summary)

- **Total Duration** - 프로파일링 대상 함수의 전체 실행 시간
- **Slowest Function** - 가장 오래 걸린 내부 함수 (빨간색 강조)
- **Functions** - 추적된 함수 호출 총 개수

### 콜 트리 (Call Tree)

각 노드에 다음 정보가 표시됩니다.

| 항목 | 설명 |
|------|------|
| **함수명** | 호출된 함수 이름 (모노스페이스 폰트) |
| **위치** | 소스 파일명과 라인 번호 |
| **소요 시간** | 해당 함수의 실행 시간 (파란색) |
| **시작/종료** | 프로파일링 시작 기준 상대 시각 |
| **라이브러리 뱃지** | 외부 라이브러리/표준 라이브러리 표시 |

- 하위 호출이 있는 노드는 **접기/펼치기**가 가능합니다.
- 가장 느린 함수는 **빨간색 배경**으로 강조됩니다.
- 외부 라이브러리 호출은 **회색 이탤릭**으로 구분됩니다.

## 활용 예시

### FastAPI 엔드포인트 프로파일링

```python
from fastapi import FastAPI
from callprofiler import profile

app = FastAPI()

@app.get("/users")
@profile
async def get_users():
    users = await db.fetch_all("SELECT * FROM users")
    return [serialize(u) for u in users]
```

### 특정 비즈니스 로직 병목 분석

```python
from callprofiler import profile

@profile
def generate_report(year: int):
    raw = fetch_annual_data(year)
    aggregated = aggregate_by_month(raw)
    charts = render_charts(aggregated)
    pdf = export_pdf(charts)
    return pdf
```

### 비동기 파이프라인 분석

```python
import asyncio
from callprofiler import profile

@profile
async def etl_pipeline():
    raw = await extract_from_source()
    transformed = await transform_data(raw)
    await load_to_warehouse(transformed)

asyncio.run(etl_pipeline())
```

## 참고 사항

- `sys.setprofile`을 사용하므로, 프로파일링 중에는 다른 프로파일러와 동시 사용이 불가합니다.
- C 확장 함수 호출(`c_call`, `c_return`)도 추적됩니다.
- callprofiler 자체의 내부 호출은 자동으로 필터링됩니다.
- 리포트 HTML은 외부 의존성 없이 단일 파일로 동작합니다.
