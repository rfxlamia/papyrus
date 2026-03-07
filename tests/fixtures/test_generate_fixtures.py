from pathlib import Path


def test_expected_fixture_files_exist():
    base = Path("tests/fixtures")
    expected = {
        "simple.pdf",
        "multi-heading.pdf",
        "bold-italic.pdf",
        "corrupted.pdf",
    }
    assert expected.issubset({p.name for p in base.glob("*.pdf")})
