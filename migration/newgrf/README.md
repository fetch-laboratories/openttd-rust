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

## Corresponding source

`ogfx-trains-0.3.0-source.tar.gz` is the unmodified GitHub archive of the
publisher's tag, downloaded on 2026-10-27 from
<https://github.com/openttdcoop/ogfx-trains/archive/refs/tags/0.3.0.tar.gz>
(repository <https://github.com/openttdcoop/ogfx-trains>, tag `0.3.0`, commit
`7de023e0049bd4d728f99a99cba4ab02eab6e24d`, tree
`8f39e6165b1ec753917c1a22ab81297060a6596d`). It holds the NML sources, the PNG
graphics, the language files, the Makefile and its scripts, and the docs.

| Item | SHA-256 |
| --- | --- |
| `ogfx-trains-0.3.0-source.tar.gz` as downloaded | `c3a68bae1b5101db0c15e9b72126e0829f5cd195b4cc851fe1618452775e389e` |
| The same archive decompressed (equal to `git archive --format=tar --prefix=ogfx-trains-0.3.0/ 7de023e0`) | `450b7fab1bdc9069b235c51c246e03f35da61921fbc74f353c6ac58b5811b6f1` |
| `ogfx-trains.grf` (shipped and rebuilt) | `70c8096a8b0aac6ea59fa0a3d78330fe6e67b86db5a7983de161ccf7c23e95c1` |

GitHub does not guarantee stable gzip bytes, so the decompressed hash is the
one to compare with a fresh download.

This source rebuilds the shipped `ogfx-trains.grf` byte for byte. The set's
readme asks for NML r1789, a revision of NML's original Mercurial history; the
git conversion at <https://github.com/OpenTTD/nml> keeps no revision numbers.
Counting changesets places r1789 within a few commits of
`cea1f25654771f5e8e98f0543372d82cfbac9180` (2012-01-08), and the rebuild used
that commit. That NML runs on Python 2.7 with PLY and PIL; the
rebuild used Python 2.7.18, PLY 3.11 and Pillow 6.2.1, with a small `Image`
module that re-exports `PIL.Image` for NML's `import Image`. The Makefile reads
the version from Mercurial, which the git tag lacks, so the build supplies the
values the shipped file carries: revision 294 (the GRF's `VRSN`) and tag
`0.3.0`. Its pattern rule expects the main `.pnml` at the top level, so the two
build runs directly from the unpacked top directory:

```sh
printf 'VERSION  :0.3.0\nTITLE    :OpenGFX+ Trains 0.3.0\nFILENAME :ogfx-trains.grf\n' > custom_tags.txt
gcc -nostdinc -D REPO_REVISION=294 -C -E - < src/ogfx-trains.pnml | sed 's/^# 0 /# 1 /' > ogfx-trains.nml
nmlc -c --grf ogfx-trains.grf ogfx-trains.nml
```

`-nostdinc` keeps GCC 5 and later from inserting `stdc-predef.h`, and the `sed`
turns their `# 0` line markers into the `# 1` that NML's lexer accepts; both
affect only preprocessor line markers, not the program.

No source was found for OpenGFX+ Road Vehicles 0.4.1 or OpenGFX+ Industries
0.3.5: their Mercurial and DevZone hosts no longer answer, and no GitHub
repository or Software Heritage origin holds them (#526).
