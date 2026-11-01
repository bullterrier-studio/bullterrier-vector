"""tools/check_font_features.py reads `cargo tree -e features`: a font crate with `std` switched on is found, one
without is not, and the crates it guards are those the workspace manifest pins.

The tree text is what cargo prints (an external boundary); the end-to-end proof, a manifest that really turns `std`
on, is in test_rust_gates.py.
"""

import importlib.util
from pathlib import Path

import pytest

pytestmark = pytest.mark.gate

ROOT = Path(__file__).resolve().parents[2]


def _load_tool():
    spec = importlib.util.spec_from_file_location("check_font_features", ROOT / "tools" / "check_font_features.py")
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tool = _load_tool()

WITHOUT_STD = """harfrust v0.15.0
harfrust feature "libm"
read-fonts feature "libm"
"""
WITH_STD = WITHOUT_STD + 'read-fonts feature "std"\n'
FROM_DEFAULT = 'read-fonts feature "default"\nread-fonts feature "std"\n'


@pytest.mark.parametrize(
    ("crate", "tree", "found"),
    [
        pytest.param("read-fonts", WITHOUT_STD, False, id="libm only"),
        pytest.param("read-fonts", WITH_STD, True, id="std switched on"),
        pytest.param("read-fonts", FROM_DEFAULT, True, id="std through default features"),
        pytest.param("harfrust", WITH_STD, False, id="std of another crate"),
        pytest.param("read-fonts", 'read-fonts-extra feature "std"\n', False, id="a crate named like it"),
    ],
)
def test_std_is_found_in_the_feature_tree_of_the_crate_that_has_it(crate, tree, found):
    assert tool.std_enabled(crate, tree) is found


def test_the_guarded_crates_are_those_the_manifest_pins_with_their_exact_versions(tmp_path):
    (tmp_path / "Cargo.toml").write_text(
        '[workspace.dependencies]\nread-fonts = { version = "=0.46.0", default-features = false }\n'
        'harfrust = "=0.15.0"\nlibm = "0.2"\n',
        encoding="utf-8",
    )
    assert tool.guarded(tmp_path) == [("read-fonts", "0.46.0"), ("harfrust", "0.15.0")]
