#!/usr/bin/env python3
"""Compare the semantic simulation state of the reference and candidate games.

Both binaries run the same headless scenarios. `-d desync=3` makes each write an
uncompressed snapshot every 32 economy days; this tool decodes every chunk of
every snapshot (table chunks field by field using the header stored in the
save, other chunks byte by byte) and reports differences as
`snapshot chunk/element/field: reference -> candidate`.

Run it through `python3 tools/migration.py simulate`, which builds both games
first. Use `--self` to compare the reference with itself (determinism and mask
check) and `--soak` for the larger scenario set. `speed prepare|collect DIR`
runs a quiet speed pass (simulation/speed.py).
"""

import sys

from simulation.core import main

if __name__ == "__main__":
    if sys.argv[1:2] == ["speed"]:
        from simulation.speed import main as speed

        sys.exit(speed(sys.argv[2:]))
    sys.exit(main())
