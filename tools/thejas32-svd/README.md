# thejas32-svd

SVD generator for CDAC VEGA THEJAS32 RISC-V SoC.

## Usage

```bash
uv run main.py                    # writes thejas32.svd in CWD
uv run main.py -o custom.svd      # writes to a custom path
```

The script always validates its output against the vendored CMSIS-SVD XSD
before exiting. On a schema violation it exits non-zero, renames the bad
file to `*.invalid`, and prints the validator's error pointing at the
offending element. There is no flag to skip validation.

## CMSIS-SVD schema validation

- Schema file: [`cmsis-svd.xsd`](./cmsis-svd.xsd) — vendored verbatim.
- CMSIS-SVD version: **1.3** (XSD file revision **1.3.12**, the latest at
  the time of vendoring).
- Origin: <https://github.com/cmsis-svd/svd-schema/blob/main/schemas/CMSIS-SVD_1_3_12.xsd>
- License: 3-clause BSD-style, Arm Limited 2013–2025. The Arm copyright
  header at the top of the XSD must be preserved.
- Validator: [`xmlschema`](https://pypi.org/project/xmlschema/) (pure
  Python, no native libxml2 dependency).

The script does **not** fetch the schema at runtime — `cmsis-svd.xsd` is
loaded from disk relative to `main.py`.

## Treating the XSD as a friend, not an obstacle

When the validator complains, the bug is almost always in `main.py`'s
peripheral builder (wrong child ordering, illegal `bitRange`, missing
required element). **Fix the generator output; never relax the
validator.** Suppressing schema errors just defers the pain to `svd2rust`
or to consumers opening the SVD in a debugger.

The first time a new peripheral type is added, validation errors form a
free checklist of what CMSIS requires — read the XSD section for the
failing element instead of guessing.

## Bumping the schema

To pick up a newer XSD revision (e.g. a future `1_3_13`):

1. Replace `cmsis-svd.xsd` with the new file from the upstream repo.
2. Update the URL above and the comment block at the top of `main.py`.
3. If the CMSIS-SVD version bumps from 1.3 → 1.4+, also update the
   `schemaVersion` attribute set in `build_device()`.
4. Re-run `uv run main.py` and fix any new violations in the generator.
