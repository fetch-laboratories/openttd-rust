#!/usr/bin/env python3
"""Check the pool-limit Split failure absent from play saves."""

import evidence_replacements
import probe


def main() -> None:
    evidence_replacements.run_rust(
        "rust:lib:cargo_storage::tests::split_denied_preserves_packet_before_share_arithmetic"
    )
    probe.passed("Cargo pool-limit Split comparison", cases=48)


if __name__ == "__main__":
    main()
