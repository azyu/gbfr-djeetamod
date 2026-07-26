import importlib.util
import struct
import tempfile
import unittest
from pathlib import Path


SCRIPT_DIR = Path(__file__).parent


def load_module(name: str):
    path = SCRIPT_DIR / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ExtractIldasmSignaturesTests(unittest.TestCase):
    def test_recovers_wildcards_and_fixed_nullable_bytes(self):
        module = load_module("extract_ildasm_signatures")
        il = """
  .method private hidebysig specialname rtspecialname static
          void .cctor() cil managed
  {
    IL_0000: ldc.i4.4
    IL_0001: newarr valuetype [System.Runtime]System.Nullable`1<uint8>
    IL_0002: dup
    IL_0003: ldc.i4.0
    IL_0004: ldc.i4 0xaa
    IL_0005: newobj instance void valuetype [System.Runtime]System.Nullable`1<uint8>::.ctor(!0)
    IL_0006: stelem valuetype [System.Runtime]System.Nullable`1<uint8>
    IL_0007: dup
    IL_0008: ldc.i4.2
    IL_0009: ldc.i4.s 16
    IL_000a: newobj instance void valuetype [System.Runtime]System.Nullable`1<uint8>::.ctor(!0)
    IL_000b: stelem valuetype [System.Runtime]System.Nullable`1<uint8>
    IL_000c: dup
    IL_000d: ldc.i4.3
    IL_000e: ldc.i4.1
    IL_000f: newobj instance void valuetype [System.Runtime]System.Nullable`1<uint8>::.ctor(!0)
    IL_0010: stelem valuetype [System.Runtime]System.Nullable`1<uint8>
    IL_0011: stsfld valuetype [System.Runtime]System.Nullable`1<uint8>[]
      Example.NativeAutomation::RewardUpdateSignature
  } // end of method NativeAutomation::.cctor
"""

        self.assertEqual(
            module.extract_signatures(il, "NativeAutomation"),
            {"RewardUpdateSignature": "AA ?? 10 01"},
        )


class ScanPeSignaturesTests(unittest.TestCase):
    def test_scans_only_executable_sections_and_returns_rva(self):
        module = load_module("scan_pe_signatures")
        image = bytearray(0x400)
        struct.pack_into("<I", image, 0x3C, 0x80)
        image[0x80:0x84] = b"PE\0\0"
        struct.pack_into("<H", image, 0x86, 1)
        struct.pack_into("<H", image, 0x94, 0xF0)
        section = 0x80 + 24 + 0xF0
        image[section : section + 8] = b".text\0\0\0"
        struct.pack_into("<IIII", image, section + 8, 0x100, 0x1000, 0x100, 0x200)
        struct.pack_into("<I", image, section + 36, 0x60000020)
        image[0x210:0x214] = bytes.fromhex("AA BB CC DD")

        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "game.exe"
            executable.write_bytes(image)
            result = module.scan_patterns(
                executable, {"reward": "AA ?? CC DD"}
            )

        self.assertEqual(
            result,
            {"reward": [{"section": ".text", "rva": 0x1010, "fileOffset": 0x210}]},
        )

    def test_duplicate_matches_are_not_treated_as_unique(self):
        module = load_module("scan_pe_signatures")
        matches = {
            "reward": [
                {"section": ".text", "rva": 1, "fileOffset": 1},
                {"section": ".text", "rva": 2, "fileOffset": 2},
            ]
        }

        self.assertEqual(module.non_unique_patterns(matches), {"reward": 2})


if __name__ == "__main__":
    unittest.main()
