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

    good = (OUT / "simple.pdf").read_bytes()
    (OUT / "corrupted.pdf").write_bytes(good[: max(100, len(good) // 4)])


if __name__ == "__main__":
    main()
