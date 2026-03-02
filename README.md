shousetsu-dump
==============

[![Rust](https://github.com/DoumanAsh/shousetsu-dump/actions/workflows/rust.yml/badge.svg)](https://github.com/DoumanAsh/shousetsu-dump/actions/workflows/rust.yml)

Web novel dumping tool. Following sites are supported:
- https://syosetu.com/
    - Including R18 available under `novel18.syosetu.com`
- https://kakuyomu.jp/


## Usage

```
Utility to download text of the web novels

Supported websites:
- kakuyomu.jp
- syosetu.com

In case of issue please file issue on https://github.com/DoumanAsh/shousetsu-dump

Always include URL to the novel alongside command line parameters you run

USAGE: [OPTIONS] <novel>

OPTIONS:
    -h,  --help                           Prints this help information
         --from <from>                    Specify from which chapter to start dumping. Default: 1.
         --to <to>                        Specify until which chapter to dump.
    -o,  --out <out>                      Output file name. By default writes ./<title>.md
         --rate <rate>                    Number of chapters to download at most per interval. Defaults to no limit.
         --rate_interval <rate_interval>  Interval between rated downloads. Defaults to 1 second.
    -d,  --dry                            Specifies to fetch novel index without download. Defaults false.

ARGS:
    <novel>  Id of the novel to dump (e.g. kakuyomu.jp/works/1177354054935164320, novel18.syosetu.com/n9598df/)
```

## Convert to EPUB

I recommend to use [pandoc](https://github.com/jgm/pandoc):

```
pandoc --metadata title="<novel title>" --embed-resources --standalone --shift-heading-level-by=-1 --from=gfm -o novel.epub "./<out>.md"
```
