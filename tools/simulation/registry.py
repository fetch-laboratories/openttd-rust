"""Scenario families and their explicitly shared hooks."""

import functools
from types import ModuleType
from typing import Literal

from .configuration import configuration
from .models import Scenario


def scenario_modules() -> tuple[ModuleType, ...]:
    """Families in scenario-list order; import after shared core initialization."""
    from . import (
        aircraft,
        cargo_storage,
        clocks,
        commands,
        community_ai,
        community_newgrf,
        companies,
        disasters,
        drawing,
        economy,
        effects,
        engines,
        fleet,
        generated,
        industries,
        landscape,
        network_protocol,
        newgrf_loading,
        newgrf_sets,
        objectives,
        objects,
        orders,
        play_saves,
        rails,
        roads,
        script_host,
        script_infrastructure,
        session,
        ships,
        signals,
        signs,
        stations,
        towns,
        trees,
        vehicles,
    )

    return (
        generated,
        landscape,
        objects,
        objectives,
        signs,
        play_saves,
        community_ai,
        community_newgrf,
        towns,
        trees,
        effects,
        engines,
        newgrf_loading,
        network_protocol,
        newgrf_sets,
        fleet,
        rails,
        ships,
        signals,
        aircraft,
        disasters,
        economy,
        clocks,
        session,
        roads,
        companies,
        commands,
        script_host,
        script_infrastructure,
        stations,
        industries,
        cargo_storage,
        orders,
        vehicles,
        drawing,
    )


def adapter_modules() -> list[ModuleType]:
    """Families whose console adapter relinks the game; runtime bundles carry each."""
    return [module for module in scenario_modules() if hasattr(module, "adapter")]


Hook = Literal["install", "prepare", "game_args", "game_environment", "check"]

# Hooks that also act on other families' scenarios. Each runs for every
# scenario and gates on the scenario's own keys; every other hook runs only for
# the scenarios its module defines.
SHARED_HOOKS: dict[str, tuple[Hook, ...]] = {
    # objects-generation-* owners
    "generated": ("prepare", "game_environment", "check"),
    # roads-flooding witnesses
    "landscape": ("game_environment", "check"),
    # play-padhattan-ridge-2000-* road witnesses from rails
    "play_saves": ("game_environment", "check"),
    # engine inputs of economy, fleet and stations; candidate engine crossings
    "engines": ("game_environment", "prepare", "check"),
    # effects-emission* fleets
    "fleet": ("prepare",),
    # unnamed controller setup inputs; disaster and play-save witnesses
    "aircraft": ("install", "game_args", "game_environment", "check"),
    # cargo storage, industry and station economies; engine monitors
    "economy": ("prepare", "check"),
    # industry-company-* companies
    "companies": ("install", "prepare", "check"),
    # unnamed reference road-construction inputs
    "roads": ("install",),
    # vehicle witnesses of aircraft, effects, fleet, play saves, rails, roads, ships
    "vehicles": ("game_environment", "check"),
}


@functools.cache
def _owners(modules: tuple[ModuleType, ...]) -> dict[str, ModuleType]:
    return {
        scenario["name"]: module
        for soak in (False, True)
        for module in modules
        for scenario in module.scenarios(soak)
    }


def hook_modules(scenario: Scenario, hook: Hook) -> list[ModuleType]:
    """Modules whose `hook` runs for this scenario, in scenario-list order: the
    module defining it and the SHARED_HOOKS entries."""
    modules = tuple(scenario_modules())
    owner = _owners(modules).get(scenario.get("name", ""))
    return [
        module
        for module in modules
        if hasattr(module, hook)
        and (
            module is owner
            or hook in SHARED_HOOKS.get(module.__name__.rsplit(".", 1)[1], ())
        )
    ]


def sanitized() -> bool:
    """Whether the driver's --sanitize selected the ASan and UBSan trees (#505)."""
    return configuration().sanitize


def scenario_list(soak: bool) -> list[Scenario]:
    return [
        scenario for module in scenario_modules() for scenario in module.scenarios(soak)
    ]
