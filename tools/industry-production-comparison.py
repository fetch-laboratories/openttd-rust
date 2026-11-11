#!/usr/bin/env python3
"""Run the absent-NewGRF industry probe in the default comparisons."""

from simulation.industries import lifecycle_oracle, production_gap

if __name__ == "__main__":
    production_gap()
    lifecycle_oracle()
