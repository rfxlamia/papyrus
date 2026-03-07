import json
import subprocess
from pathlib import Path


def test_oracle_emits_pages_and_blocks(tmp_path):
    pdf = Path("tests/fixtures/simple.pdf")
    out = tmp_path / "simple.oracle.json"
    subprocess.run(
        ["python3", "tests/oracle/extract_oracle.py", str(pdf), "--out", str(out)],
        check=True,
    )
    data = json.loads(out.read_text())
    assert "pages" in data
    assert isinstance(data["pages"], list)
