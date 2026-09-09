# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.1.0](https://github.com/bosun-ai/smackdebt/releases/tag/v0.1.0) - 2026-09-09

Install the CLI and agent skill:

```sh
curl -fsSL https://github.com/bosun-ai/smackdebt/releases/download/v0.1.0/install.sh | sh
```

For just the CLI, add `-s -- --no-skill` after `sh`.

### Added

- add C# analysis ([#6](https://github.com/bosun-ai/smackdebt/pull/6))

- add PHP analysis

- add Go analysis

- add agent skill and one-line installation ([#3](https://github.com/bosun-ai/smackdebt/pull/3))

- *(history)* state the concentration a change moved, not the standing fact

- *(gate)* name the vanished counter a regression may have come from

- *(diff)* follow a unit the change moved between files

- *(report)* keep the base-side name a renamed file answered to

- *(output)* make a diff answer for its own change

- *(project)* classify vendored javascript out of the verdict

- *(analysis)* add a vendored source role no verdict reads

- *(output)* state how far a typical change travels at every scope

- *(output)* say when the largest cycle is the repository's core

- *(analysis)* let hub cards fire only with co-change proof

- *(analysis)* compare trusted graph movement

- *(analysis)* name only the leaks a reader can act on

- *(analysis)* find interfaces whose importers follow their changes

- *(analysis)* state how many files a typical change touches

- *(analysis)* count file pairs that change together

- *(analysis)* state how far a change can reach

- *(analysis)* state a sub-scope's share of the repository debt

- *(output)* render ranked problem cards under a one-screen budget

- *(output)* serialize problem cards in the v4 report

- *(output)* stop printing dependency edges as rows

- *(cli)* ratchet smackdebt against its own baseline

- *(cli)* add a ratchet gate over a committed baseline

- *(cli)* add --top for a middle level of detail

- *(analysis)* state the unsupported source share in the verdict

- *(analysis)* weigh debt volume and evidence in the codebase tier

- *(output)* say why imports could not be followed

- *(output)* say when the history window is empty

- *(discovery)* exclude nested git checkouts and record them

- *(project)* classify test-declared module files as test source

- *(project)* build architecture verdicts from primary relations only

- *(output)* state the present side of added and removed cards

- *(analysis)* rank production debt before test debt

- *(output)* adopt report schema version 4

- *(analysis)* rate maximum nesting depth and parameter count

- *(cli)* give the report exact errors, decorations, and rewritten docs

- *(output)* make words carry the terminal verdict and decorate them

- *(analysis)* add hotspots, hot and role rank keys, and size findings

- *(evolution)* govern every history signal with the selected window

- simplify terminal output

- publish analysis report v3

- strengthen evolution evidence

- distinguish static relation evidence

- classify analysis evidence

- add evolutionary architecture analysis

- add static architecture analysis

- replace source analysis engine


### Fixed

- *(output)* warn a diff only about the files it measured

- *(output)* let the diff pointer read the whole ranking

- *(output)* make every terminal line say what it means

- *(output)* point a diff only at paths the change left behind

- *(analysis)* withhold only the unpaired units that answer for each other

- *(languages)* anchor ruby dsl blocks on their call chain

- *(project)* analyze the repository for any selected scope

- *(analysis)* withhold the reach and core facts the evidence counts hide

- *(languages)* let an invented closer reach the block it grew

- *(languages)* trust recovered parses whose errors touch nothing measured

- *(project)* decide assetness on candidate paths, not written targets

- *(project)* classify asset imports instead of calling them unresolved

- *(analysis)* prioritize primary application problems

- *(output)* make diff conclusions neutral and representative

- *(discovery)* classify generated javascript assets

- *(analysis)* match moved anonymous units

- *(project)* report unsupported selected source

- *(output)* name the file a claimed hidden pair changed with

- *(analysis)* keep a component index eligible to leak

- *(analysis)* exclude the re-export surfaces, not every entry file

- *(analysis)* let a one-package repository state its own reach

- *(analysis)* state a tangle's witness before its member count

- *(output)* state a warning detail row's path once

- *(output)* exempt cycle witnesses from the budget and agree with every count

- *(languages)* scrub every dependency target at construction, not per-splitter

- *(languages)* record dynamic import targets on one line

- *(analysis,output)* distinguish no dependency from an indirect one

- *(project)* exclude module-owned pairs from the file cycle graph

- *(cli)* exit quietly when the reader closes standard output

- *(output)* answer the selected scope in the JSON head

- *(schema)* allow a healthy maximum rating on a hotspot row

- *(output)* reserve the decoration cells so both audiences share one layout

- *(analysis)* keep one coupling row per package pair and drop nested pairs

- call logical lines statements

- make evidence gate feature stable

- exclude generated Rails schema debt


### Other

- *(analysis)* organize and document metrics by name ([#2](https://github.com/bosun-ai/smackdebt/pull/2))

- prepare GitHub binary releases and shorten the README

- *(project)* take one health policy where five tuples traveled

- *(cli)* resolve the selected path once and extract rendering

- pin the uncovered diff and gate flows before decomposition

- *(readme)* re-capture the drifted examples and settle the hub rule

- *(analysis)* call measured inattention dormant, not vendored

- *(readme)* correct what reach counts and when a core goes unstated

- *(readme)* document where each Ousterhout fact is stated

- finish the openspec removal

- re-capture the numbers the wave's own commits moved

- *(cli)* prove the windowed arm's sample is empty rather than shallow

- *(analysis)* mirror the file pair contract into both checkers

- re-capture the README examples the wave moved

- *(cli)* discharge the problem-card evidence the delta still owed

- describe the problem-card terminal and JSON

- *(output)* guard the architecture severity sort and the last run helper

- *(git)* ask git for the history window

- *(discovery)* walk the tree through the ignore crate with git semantics

- *(cli)* pin the acceptance child environment to a hermetic home

- *(repo)* add the MIT license, retire the dead evidence map, and make JSON snapshots reviewable

- *(cli)* read report paths and fixture commits through one helper each

- explain primary-only architecture verdicts and Rust module wiring

- *(cli)* pin primary-only verdict graphs in committed evidence

- *(cli)* pin test-scoped relations in committed json evidence

- state the non-primary worst-offender fallback

- *(cli)* pin the production-first rank order in public bytes

- describe schema version 4 and the five rated measurements

- *(cli)* move focused acceptance evidence onto the words-based report

- record the deepened signals in the architecture guide and report

- *(cli)* prove hot production debt outranks cold and test debt

- *(readme)* document the new rank keys and the governing history window

- explain workspace name resolution and one coupling row per pair

- *(cli)* prove workspace manifest resolution end to end

- add unified analysis evidence

- Cleanup

- Prettier output

- Bit nicer

- Initial draft

- Initial spec

