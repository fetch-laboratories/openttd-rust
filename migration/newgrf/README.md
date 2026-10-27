# File-loaded NewGRF sets (#507)

Agent: /root/corpus507_newgrf_sets | Model: claude-opus-5-5 | Reasoning effort: high

Inputs for the `newgrf-sets-*` scenarios (`tools/simulation/newgrf_sets.py`).
Each `.grf` is the unmodified file from its BaNaNaS package, downloaded on
2026-10-27 through the pinned reference build's own content client (`content
select` and `content download`), which is the only distribution channel the
sets offer. Each package states GPL version 2 or later in its readme, which is
kept here with its credits as `<name>-readme.txt` (trailing spaces removed for
the commit checker). All three packages ship the same GPLv2 text, kept once as
`license.txt`.

| File | Package (BaNaNaS unique ID) | Version | SHA-256 |
| --- | --- | --- | --- |
| `ogfx-trains.grf` | OpenGFX+ Trains (`4F472B31`) | 0.3.0 | `70c8096a8b0aac6ea59fa0a3d78330fe6e67b86db5a7983de161ccf7c23e95c1` |
| `ogfx-rv.grf` | OpenGFX+ Road Vehicles (`4F472B32`) | 0.4.1 | `0dac319842b6ddc3cb0bb31116bbe18798a4b07bbb40c2689c89485de04534b1` |
| `ogfx-industries.grf` | OpenGFX+ Industries (`4F472B33`) | 0.3.5 | `83fc6eed5d72b08bc6d4409d04b07330d5803caccda6e575721f7f9e0880ff86` |

The industries package names its folder `OpenGFX_Industries-VERSION`; its readme
identifies version 0.3.5. `license.txt` has SHA-256
`ab15fd526bd8dd18a9e77ebc139656bf4d33e97fc7238cd11bf60e2b9b8666c6`.

FIRS Industries 5 (5.2.0, 9 MB) and Iron Horse 4 (4.36.0, 58 MB) are also GPLv2
but were left out for their size.
