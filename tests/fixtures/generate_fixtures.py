from pathlib import Path
import fitz

OUT = Path(__file__).resolve().parent


def _write_pdf(path: Path, lines):
    doc = fitz.open()
    page = doc.new_page()
    y = 72
    for text, size, font in lines:
        page.insert_text((72, y), text, fontsize=size, fontname=font)
        y += size + 10
    doc.save(path)
    doc.close()


def _write_multi_page_pdf(path: Path, pages_lines):
    """Write a PDF with multiple pages. pages_lines is a list of lists of (text, size, font)."""
    doc = fitz.open()
    for lines in pages_lines:
        page = doc.new_page()
        y = 72
        for text, size, font in lines:
            page.insert_text((72, y), text, fontsize=size, fontname=font)
            y += size + 10
    doc.save(path)
    doc.close()


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    _write_pdf(
        OUT / "simple.pdf", [("Chapter 1", 24, "helv"), ("Body text.", 12, "helv")]
    )
    _write_pdf(
        OUT / "multi-heading.pdf",
        [("H1", 28, "helv"), ("H2", 22, "helv"), ("Body", 12, "helv")],
    )
    _write_pdf(OUT / "bold-italic.pdf", [("Bold", 14, "hebo"), ("Italic", 14, "heit")])
    _write_multi_page_pdf(
        OUT / "multi-page.pdf",
        [
            [("Page 1 Title", 24, "helv"), ("Page 1 body.", 12, "helv")],
            [("Page 2 Title", 24, "helv"), ("Page 2 body.", 12, "helv")],
        ],
    )

    good = (OUT / "simple.pdf").read_bytes()
    (OUT / "corrupted.pdf").write_bytes(good[: max(100, len(good) // 4)])


if __name__ == "__main__":
    main()
