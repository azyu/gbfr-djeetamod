#!/usr/bin/env python3
import argparse
import json
import struct
from pathlib import Path


IMAGE_SCN_MEM_EXECUTE = 0x20000000


def executable_sections(image: bytes):
    if len(image) < 0x40:
        raise ValueError("file is too small for a PE image")
    pe_offset = struct.unpack_from("<I", image, 0x3C)[0]
    if image[pe_offset : pe_offset + 4] != b"PE\0\0":
        raise ValueError("missing PE signature")
    section_count = struct.unpack_from("<H", image, pe_offset + 6)[0]
    optional_size = struct.unpack_from("<H", image, pe_offset + 20)[0]
    section_table = pe_offset + 24 + optional_size

    sections = []
    for index in range(section_count):
        offset = section_table + index * 40
        if offset + 40 > len(image):
            raise ValueError("truncated PE section table")
        name = image[offset : offset + 8].split(b"\0", 1)[0].decode(
            "ascii", errors="replace"
        )
        _, rva, raw_size, raw_offset = struct.unpack_from("<IIII", image, offset + 8)
        characteristics = struct.unpack_from("<I", image, offset + 36)[0]
        if characteristics & IMAGE_SCN_MEM_EXECUTE:
            sections.append((name, rva, raw_offset, raw_size))
    return sections


def parse_pattern(text: str):
    tokens = text.split()
    if not tokens:
        raise ValueError("empty signature")
    return [None if token == "??" else int(token, 16) for token in tokens]


def longest_literal_run(pattern):
    best_start = best_length = current_start = current_length = 0
    for index, byte in enumerate(pattern):
        if byte is None:
            current_length = 0
            current_start = index + 1
        else:
            current_length += 1
            if current_length > best_length:
                best_start, best_length = current_start, current_length
    if best_length == 0:
        raise ValueError("signature cannot contain only wildcards")
    return best_start, bytes(pattern[best_start : best_start + best_length])


def scan_one(image: bytes, sections, pattern_text: str):
    pattern = parse_pattern(pattern_text)
    anchor_offset, anchor = longest_literal_run(pattern)
    hits = []

    for section_name, section_rva, raw_offset, raw_size in sections:
        blob = image[raw_offset : raw_offset + raw_size]
        start = 0
        while True:
            anchor_position = blob.find(anchor, start)
            if anchor_position < 0:
                break
            candidate = anchor_position - anchor_offset
            if candidate >= 0 and candidate + len(pattern) <= len(blob):
                if all(
                    wanted is None or blob[candidate + index] == wanted
                    for index, wanted in enumerate(pattern)
                ):
                    hits.append(
                        {
                            "section": section_name,
                            "rva": section_rva + candidate,
                            "fileOffset": raw_offset + candidate,
                        }
                    )
            start = anchor_position + 1
    return hits


def scan_patterns(executable: Path, patterns):
    image = executable.read_bytes()
    sections = executable_sections(image)
    return {
        name: scan_one(image, sections, pattern)
        for name, pattern in sorted(patterns.items())
    }


def non_unique_patterns(matches):
    return {name: len(hits) for name, hits in matches.items() if len(hits) != 1}


def main():
    parser = argparse.ArgumentParser(
        description="Scan executable PE sections for masked byte signatures."
    )
    parser.add_argument("--exe", required=True, type=Path)
    parser.add_argument("--patterns", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-unique", action="store_true")
    args = parser.parse_args()

    patterns = json.loads(args.patterns.read_text(encoding="utf-8"))
    matches = scan_patterns(args.exe, patterns)
    summary = {
        name: {
            "count": len(hits),
            "matches": hits,
        }
        for name, hits in matches.items()
    }
    payload = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(payload, encoding="utf-8")
    else:
        print(payload, end="")

    if args.require_unique and non_unique_patterns(matches):
        raise SystemExit(2)


if __name__ == "__main__":
    main()
