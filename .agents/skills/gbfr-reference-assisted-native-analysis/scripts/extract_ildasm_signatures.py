#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path


LDC_NUMBER = re.compile(r"\bldc\.i4(?:\.s)?\s+(0x[0-9a-fA-F]+|-?\d+)")
LDC_SHORT = re.compile(r"\bldc\.i4\.([0-8])(?:\s|$)")
FIELD = re.compile(r"::([A-Za-z0-9_]+Signature)\s*$")


def parse_ldc(line: str):
    if re.search(r"\bldc\.i4\.m1(?:\s|$)", line):
        return -1
    short = LDC_SHORT.search(line)
    if short:
        return int(short.group(1))
    number = LDC_NUMBER.search(line)
    if not number:
        return None
    value = number.group(1)
    return int(value[2:], 16) if value.lower().startswith("0x") else int(value)


def extract_signatures(il_text: str, class_name: str):
    in_cctor = False
    values = []
    array = None
    awaiting_field = False
    signatures = {}

    for line in il_text.splitlines():
        if re.search(r"\bvoid\s+\.cctor\(\)", line):
            in_cctor = True
            continue
        if not in_cctor:
            continue
        if f"end of method {class_name}::.cctor" in line:
            break

        value = parse_ldc(line)
        if value is not None:
            values.append(value)

        if "newarr" in line and "Nullable`1<uint8>" in line:
            if not values or values[-1] < 0:
                raise ValueError("nullable-byte array has no valid length")
            array = [None] * values[-1]
            values.clear()
            continue

        if array is not None and "stelem" in line and "Nullable`1<uint8>" in line:
            if len(values) < 2:
                raise ValueError("nullable-byte array element is missing index or value")
            index, byte = values[-2], values[-1]
            if not 0 <= index < len(array):
                raise ValueError(f"signature index {index} is outside array length {len(array)}")
            array[index] = byte & 0xFF
            values.clear()
            continue

        if array is not None and "stsfld" in line:
            awaiting_field = True

        if array is not None and awaiting_field:
            field = FIELD.search(line)
            if field:
                signatures[field.group(1)] = " ".join(
                    "??" if byte is None else f"{byte:02X}" for byte in array
                )
                array = None
                awaiting_field = False
                values.clear()

    if not signatures:
        raise ValueError(f"no nullable-byte signatures found in {class_name}::.cctor")
    return signatures


def main():
    parser = argparse.ArgumentParser(
        description="Extract masked byte signatures from ildasm nullable-byte arrays."
    )
    parser.add_argument("--il", required=True, type=Path)
    parser.add_argument("--class-name", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    signatures = extract_signatures(
        args.il.read_text(encoding="utf-8", errors="replace"), args.class_name
    )
    payload = json.dumps(signatures, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(payload, encoding="utf-8")
    else:
        print(payload, end="")


if __name__ == "__main__":
    main()
