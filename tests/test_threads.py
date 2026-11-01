"""A draw does not hold up the other Python threads: a preview server draws on a worker thread while its main thread
keeps answering. The test is that a thread waiting in Python gets to run during a draw, not how long the draw takes."""

import sys
import threading
from collections.abc import Callable, Iterator

import pytest

import bullterrier_vector as btvec

# Heavy enough that a draw lasts many times as long as a thread needs to wake up.
CIRCLES = btvec.Drawing(*(btvec.Path.circle((i % 200, i // 200), 5).fill("#336699") for i in range(20_000)))
DOCUMENT = CIRCLES.to_json()

DRAWS: list[tuple[str, Callable[[], object]]] = [
    ("to_svg", lambda: CIRCLES.to_svg(2000, 2000)),
    ("to_rgba", lambda: CIRCLES.to_rgba(3000, 3000)),
    ("to_png", lambda: CIRCLES.to_png(3000, 3000)),
    ("to_mesh", lambda: CIRCLES.to_mesh(2000, 2000)),
    ("shapes", lambda: btvec.Drawing(CIRCLES, CIRCLES.translate(1, 1)).shapes()),
    ("from_json", lambda: btvec.Drawing.from_json(DOCUMENT)),
]


@pytest.fixture
def no_timed_handoff() -> Iterator[None]:
    """Make the interpreter hand its lock to a waiting thread only when the running one lets go of it (a native call
    that detaches, or a blocking call), not after a few milliseconds of bytecode: without this, the waiting thread
    would run at the first bytecode after a draw that kept the lock, and look as if it had run during it."""
    interval = sys.getswitchinterval()
    sys.setswitchinterval(1000.0)
    try:
        yield
    finally:
        sys.setswitchinterval(interval)


@pytest.mark.usefixtures("no_timed_handoff")
@pytest.mark.parametrize("draw", [pytest.param(call, id=name) for name, call in DRAWS])
def test_another_python_thread_runs_while_a_draw_is_in_progress(draw: Callable[[], object]):
    go, ran = threading.Event(), threading.Event()

    def wait_then_run() -> None:
        go.wait()
        ran.set()

    waiting = threading.Thread(target=wait_then_run)
    waiting.start()
    try:
        go.set()
        draw()
        ran_during_the_draw = ran.is_set()
    finally:
        go.set()
        waiting.join()
    assert ran_during_the_draw
