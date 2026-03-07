import argparse
import json
from pathlib import Path
import fitz


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("pdf", type=Path)
    p.add_argument("--out", type=Path, required=True)
    return p.parse_args()


def block_to_record(span):
    text = span.get("text", "").strip()
    if not text:
        return None
    font_name = span.get("font")
    flags = span.get("flags", 0)
    return {
        "text": text,
        "font_name": font_name,
        "font_size": float(span.get("size", 0.0)),
        "is_bold": bool(flags & (1 << 4)),
        "is_italic": bool(flags & (1 << 1)),
    }


def extract(pdf_path: Path):
    doc = fitz.open(pdf_path)
    pages = []
    for i, page in enumerate(doc):
        page_blocks = []
        for block in page.get_text("dict").get("blocks", []):
            for line in block.get("lines", []):
                for span in line.get("spans", []):
                    record = block_to_record(span)
                    if record:
                        page_blocks.append(record)
        pages.append({"page_number": i, "blocks": page_blocks})
    doc.close()
    return {"pages": pages}


def main():
    args = parse_args()
    payload = extract(args.pdf)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
