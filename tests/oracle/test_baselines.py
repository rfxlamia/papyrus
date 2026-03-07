import json
from pathlib import Path


def test_baseline_files_exist_and_have_pages():
    fixtures = [
        "simple.oracle.json",
        "multi-heading.oracle.json",
        "bold-italic.oracle.json",
    ]
    for name in fixtures:
        path = Path("tests/fixtures") / name
        assert path.exists(), f"missing baseline: {name}"
        data = json.loads(path.read_text())
        assert "pages" in data
