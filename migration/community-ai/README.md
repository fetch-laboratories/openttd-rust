# Community AI inputs (#507)

Agent: /root/implement507_ai_large_corpus | Model: gpt-6.1-sol | Reasoning effort: high.

`packages.json` pins the complete source packages served by OpenTTD's official
BaNaNaS content service. It records the exact upload metadata, publisher,
version, content MD5, archive SHA-256, license and transitive dependencies.
The archive URL is
`https://bananas-cdn.openttd.org/{kind}/{uid}/{md5}/{uid}-{name}-{version}.tar.gz`.
Download and validate them with:

```sh
python3 -m tools.simulation.community_ai fetch
```

Archives remain uncommitted in ignored storage. Every use verifies SHA-256,
and an invalid cached archive fails. CI uses the ordinary Actions cache keyed
by the manifest; a cache miss downloads the exact package. Missing inputs fail
the simulation. Runtime installation decompresses complete archives to `.tar`
without stripping their files or notices. These are interpreted Squirrel sources,
not compiled programs: their runnable packages contain the corresponding `.nut`
sources, GPLv2 license texts and all packaged notices. No source offer is made.

| Package | Version | License | Archive SHA-256 |
| --- | --- | --- | --- |
| [SimpleAI](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai/534d504c/versions/20190120T143508Z.yaml) | 14 | GPL-2.0-only | `dba08d42e0c712584509634b99b8ce4970e02336cfe02aa1133ea9e6cf89754a` |
| [AdmiralAI](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai/41444d4c/versions/20100812T231406Z.yaml) | 25 | GPL-2.0-or-later | `d976e6e440dc832c88a31c446411bc1786e75623e8edcb7b42ce2c9d814c016c` |
| [trAIns](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai/54524149/versions/20140803T230851Z.yaml) | 2.1 | GPL-2.0-or-later | `8f8553e61e6115bcbff9e2ed2f635eee128c79911afa5ca2e19cb877ad9d7110` |
| [Pathfinder.Rail](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai-library/5046524c/versions/20090117T163549Z.yaml) | 1 | GPL-2.0-only | `aee7bac9259851b11e391de517542b5eb0a3a4a3dae7a0c9b055f5f66a6d5434` |
| [Pathfinder.Road](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai-library/5046524f/versions/20120819T204911Z.yaml) | 4 | GPL-2.0-only | `a26081bcbec2846a3e1bdb617cda0f957541e352337e8905a3340ce82683a3a9` |
| [Queue.FibonacciHeap](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai-library/51554648/versions/20090615T114617Z.yaml) | 2 | GPL-2.0-only | `19849ccef9cbc8dd63558b1d58224ee9f860d4d355379c7f6f758703504a917f` |
| [Graph.AyStar](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai-library/4752412a/versions/20090117T163452Z.yaml) | 4 | GPL-2.0-only | `e84be4afae4953b48318b31137208fff4a9948afbb5efb32ca5e8ab2df0eadc7` |
| [Graph.AyStar](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai-library/4752412a/versions/20090605T220117Z.yaml) | 6 | GPL-2.0-only | `0482bc32839e28210a6a371097b826cd59923b9db246922de9096051b07d4efd` |
| [Queue.BinaryHeap](https://github.com/OpenTTD/BaNaNaS/blob/75a904f4fe1e7bc8cde4c272462fcbaae181e836/ai-library/51554248/versions/20090117T163656Z.yaml) | 1 | GPL-2.0-only | `474461d9cea10b41f9e62945d2482b266dfa91ea70b5f41d8654e513e7331e3d` |

SimpleAI is Brumi's GPLv2 script; its readme states version 2, and its Terraform
helper retains krinn's GPLv2-or-later notice. AdmiralAI retains Thijs Marinussen's
GPLv2-or-later notices. trAIns retains Luis Henrique O. Rios's GPLv2-or-later
notices. The six library packages retain the OpenTTD NoAI Developers Team's
authorship in `library.nut` and their GPLv2 license text. All nine archives contain
only source and text files; no source build or additional binary asset is needed.

SimpleAI imports Pathfinder.Road 4 and Pathfinder.Rail 1, which import
Graph.AyStar 6 and 4 respectively. Both AyStar versions import Queue.BinaryHeap 1.
AdmiralAI imports Queue.FibonacciHeap 2. trAIns has no external library import.
The installed set contains exactly those versions. The releases declare API
versions 1.2, 1.0 and 1.0 respectively; OpenTTD's compatibility scripts remain
part of each normal runtime. trAIns' upload version is 2.1, while its `GetVersion`
returns 2 and its `CanLoadFromVersion` returns false. Loading a saved trAIns
company therefore restarts that AI as the original does; the other AIs use their
own Save/Load implementations. The save compares their opaque AIPL data too.

The module registers one default generated case (two years, seed 507, temperate,
256x256, three community AIs) and one short fixed-save reload. With `--soak`,
the generated games run ten years in temperate and arctic, and the fixed-save
reload runs two years. Years budget 366 days, covering leap days. Both ordinary
and snapshot modes compare every semantic checkpoint and end state through the
canonical harness. Complete script debug output and complete command/debug logs
compare after removing only wall-clock log prefixes and each run directory.
The command log is retained intact; the result records its normalized SHA-256.
Checks require each AI to construct in generated games and earn transport
revenue with cargo deliveries. Snapshot/reload checks require vehicle movement.
The result reports transport inventory, delivered cargo, revenue, observed loans,
bankruptcy months and successful command counts. These distinguish observed
activity from an AI author's advertised capabilities. No particular bankruptcy,
merger or ship activity is promised.

```sh
python3 tools/migration.py simulate community-ai --self --jobs 2
python3 tools/migration.py simulate community-ai --jobs 2
python3 tools/migration.py simulate community-ai --soak --self --jobs 2
python3 tools/migration.py simulate community-ai --soak --jobs 2
```

Generate the vanilla 256x256 arctic fixed save using only the unchanged pinned
original, after twenty years of play by these three AIs:

```sh
python3 tools/migration.py build --jobs 2
python3 -m tools.simulation.community_ai generate
```

The generator freezes the original runtime under the existing reference lock,
uses the ordinary harness configuration/launcher and saves the final original
state with LZMA compression. The sibling JSON provenance records the original
revision, configuration, generation command, manifest hash, end moment and save
hash. The save is a GPLv2-compatible generated fixture; it contains saved AI data,
not embedded script source. Reload installs the same pinned source packages,
removes only optional GLOG using the existing byte-checked normalization, and
compares original/candidate snapshots and ordinary end states. No map, vehicle,
finance or RNG field is patched. Regeneration need not have identical bytes:
the canonical comparison retains its existing random-save-ID and GLOG masks.

NewGRF input and source packaging (#525/#526) remain user-parked. This corpus
uses vanilla vehicles/industries. The eventual larger save using the same
NewGRFs as a future corpus remains a limit of #507.
