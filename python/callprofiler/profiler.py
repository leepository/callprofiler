"""Profiling decorator and sys.setprofile hook."""

import functools
import os
import sys
import sysconfig
import time
from pathlib import Path
from typing import Any, Callable

from callprofiler._callprofiler import process_events

_SITE_PACKAGES_MARKERS: tuple[str, ...] = ("site-packages", "dist-packages")
_FROZEN_PREFIX = "<frozen"


def _compute_stdlib_paths() -> tuple[str, ...]:
    paths = []
    for key in ("stdlib", "platstdlib"):
        p = sysconfig.get_paths().get(key)
        if p:
            paths.append(os.path.normpath(p))
    paths.append(os.path.normpath(sys.prefix))
    return tuple(paths)


_STDLIB_PATHS: tuple[str, ...] = _compute_stdlib_paths()


def _is_external(filename: str | None) -> bool:
    if filename is None:
        return True
    if filename.startswith(_FROZEN_PREFIX) or filename.startswith("<"):
        return True
    norm = os.path.normpath(filename)
    for marker in _SITE_PACKAGES_MARKERS:
        if marker in norm:
            return True
    for sp in _STDLIB_PATHS:
        if norm.startswith(sp):
            return True
    return False


def _extract_library_name(filename: str | None, module: str | None) -> str:
    if module:
        return module.split(".")[0]
    if filename is None:
        return "<builtin>"
    for marker in _SITE_PACKAGES_MARKERS:
        idx = filename.find(marker)
        if idx != -1:
            rest = filename[idx + len(marker) :].lstrip(os.sep)
            return rest.split(os.sep)[0].split(".")[0]
    return "<stdlib>"


def profile(
    func: Callable | None = None, *, output_dir: str = ".profile"
) -> Callable:
    """Decorator to profile a function and generate an HTML call graph.

    Usage:
        @profile
        def my_api_endpoint():
            ...

        @profile(output_dir="custom_dir")
        def my_api_endpoint():
            ...
    """
    if func is None:
        return lambda f: profile(f, output_dir=output_dir)

    @functools.wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        events: list[dict] = []
        start_ns = time.perf_counter_ns()
        profiler_module = __name__

        def _profile_callback(frame: Any, event: str, arg: Any) -> None:
            ts = time.perf_counter_ns()

            if event in ("call", "return"):
                code = frame.f_code
                filename = code.co_filename
                mod = frame.f_globals.get("__name__", "")

                # Skip callprofiler's own frames
                if mod == profiler_module:
                    return

                is_ext = _is_external(filename)
                events.append(
                    {
                        "event": event,
                        "func_name": code.co_name,
                        "module": mod,
                        "filename": filename,
                        "lineno": code.co_firstlineno,
                        "timestamp_ns": ts,
                        "is_external": is_ext,
                        "library_name": (
                            _extract_library_name(filename, mod) if is_ext else ""
                        ),
                    }
                )
            elif event in ("c_call", "c_return"):
                c_func_name = getattr(arg, "__name__", str(arg))
                c_module = getattr(arg, "__module__", "") or ""

                # Skip sys.setprofile itself (called during teardown)
                if c_func_name == "setprofile":
                    return

                events.append(
                    {
                        "event": event,
                        "func_name": c_func_name,
                        "module": c_module,
                        "filename": "",
                        "lineno": 0,
                        "timestamp_ns": ts,
                        "is_external": True,
                        "library_name": (
                            c_module.split(".")[0] if c_module else "builtins"
                        ),
                    }
                )

        sys.setprofile(_profile_callback)
        try:
            result = func(*args, **kwargs)
        finally:
            sys.setprofile(None)
            end_ns = time.perf_counter_ns()

            if events:
                html = process_events(events, func.__name__, start_ns, end_ns)

                out_path = Path(output_dir)
                out_path.mkdir(parents=True, exist_ok=True)
                timestamp = time.strftime("%Y%m%d_%H%M%S")
                report_file = out_path / f"{func.__name__}_{timestamp}.html"
                report_file.write_text(html, encoding="utf-8")
                print(f"[callprofiler] Report saved to {report_file}")

        return result

    return wrapper
