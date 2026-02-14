//! Web novel dumping utilities

mod cli;
use shousetsu_dump::stdio::Stdio;
use shousetsu_dump::{novel, http};

use std::process::ExitCode;
use core::num::NonZeroUsize;

fn args_from_stdin(stdio: &Stdio) -> Result<cli::Cli, ExitCode> {
    let mut stdin = stdio.stdin();
    let mut stdout = stdio.stdout().ignore_errors();
    let mut stderr = stdio.stderr().ignore_errors();

    macro_rules! prompt {
        ($($arg:tt)*) => {
            stdout.write_fmt(format_args!($($arg)*));
        }
    }
    macro_rules! read_line {
        () => {
            match stdin.read_line() {
                Ok(line) => line.trim(),
                Err(error) => {
                    stderr.write_fmtn(format_args!("!>>>Unexpected I/O error: {error}"));
                    return Err(ExitCode::FAILURE);
                }
            }
        };
    }

    let novel;
    loop {
        prompt!(">Please input novel id (e.g. kakuyomu.jp/works/1177354054935164320, novel18.syosetu.com/n9598df/): ");
        let line = read_line!();
        if line.is_empty() {
            continue;
        }

        if let Some(new_id) = novel::Id::try_parse(line) {
            novel = new_id;
            break;
        } else {
            stderr.write_fmtn(format_args!("!>>>Unable to recognize novel format"));
        }
    }

    let from;
    prompt!(">Please specify which chapters to download:\n");
    loop {
        prompt!("Start FROM chapter(defaults to 1)?:");
        let line = read_line!();
        if line.is_empty() {
            from = cli::default_from_value();
            break;
        }

        match line.parse() {
            Ok(chapter) => match NonZeroUsize::new(chapter) {
                Some(chapter) => {
                    from = chapter;
                    break;
                },
                None => {
                    stderr.write_fmtn(format_args!("!>>>Chapter cannot be zero"));
                    continue
                }
            },
            Err(error) => {
                stderr.write_fmtn(format_args!("!>>>'{line}': {error}"));
                continue;
            }
        }
    }

    let to;
    loop {
        prompt!("TO chapter(leave empty for all)?:");
        let line = read_line!();
        if line.is_empty() {
            to = None;
            break;
        }

        match line.parse() {
            Ok(chapter) => if chapter > from.get() {
                to = Some(unsafe {
                    NonZeroUsize::new_unchecked(chapter)
                });
                break;
            } else {
                stderr.write_fmtn(format_args!("!>>>Number has to be greater than from='{from}'"));
                continue
            },
            Err(error) => {
                stderr.write_fmtn(format_args!("!>>>{error}"));
                continue;
            }
        }
    }

    let rate;
    loop {
        prompt!("Rate limit on number of chapters to be downloaded per second(leave empty for no)?:");
        let line = read_line!();
        if line.is_empty() {
            rate = 0;
            break;
        }

        match line.parse() {
            Ok(new_value) => {
                rate = new_value;
                break;
            },
            Err(error) => {
                stderr.write_fmtn(format_args!("!>>>{error}"));
                continue;
            }
        }
    }

    let rate_interval;
    loop {
        prompt!("Interval between rate limited downloads(leave empty for 1s)?:");
        let line = read_line!();
        if line.is_empty() {
            rate_interval = 1;
            break;
        }

        match line.parse() {
            Ok(new_value) => {
                rate_interval = new_value;
                break;
            },
            Err(error) => {
                stderr.write_fmtn(format_args!("!>>>{error}"));
                continue;
            }
        }
    }


    prompt!(">Specify output file (leave empty for default): ");
    let line = read_line!();
    let out = if line.is_empty() {
        None
    } else {
        Some(line.to_owned())
    };

    stdout.write_newline();

    Ok(cli::Cli {
        from,
        to,
        out,
        novel,
        rate,
        rate_interval,
    })
}

fn run_syosetu(stdio: Stdio, args: cli::Cli) -> ExitCode {
    let http = http::Client::new();
    let mut stderr = stdio.stderr().ignore_errors();
    let mut stdout = stdio.stdout().ignore_errors();

    stdout.write_fmtn(format_args!("{}: Fetch novel index...", args.novel.url()));
    let info = loop {
        match novel::fetch_syosetu(args.novel.clone(), &http) {
            Ok(info) => break info,
            Err(novel::Error::Transient(error)) => {
                stdout.write_fmtn(format_args!("ERR"));
                stderr.write_fmtn(format_args!("{:?}", error));
            }
            Err(error) => {
                stdout.write_fmtn(format_args!("ERR"));
                stderr.write_fmtn(format_args!("Unable to fetch novel: {error}"));
                return ExitCode::FAILURE
            }
        }
    };

    stdout.write_fmtn(format_args!("{:#?}", info));
    ExitCode::SUCCESS
}

fn run_kakuyomu(stdio: Stdio, args: cli::Cli) -> ExitCode {
    let http = http::Client::new();
    let mut stderr = stdio.stderr().ignore_errors();
    let mut stdout = stdio.stdout().ignore_errors();

    stdout.write_fmtn(format_args!("{}: Fetch novel index...", args.novel.url()));
    let info = loop {
        match novel::fetch_kakuyomu(args.novel.clone(), &http) {
            Ok(info) => break info,
            Err(novel::Error::Transient(error)) => {
                stdout.write_fmtn(format_args!("ERR"));
                stderr.write_fmtn(format_args!("{error}"));
            }
            Err(error) => {
                stdout.write_fmtn(format_args!("ERR"));
                stderr.write_fmtn(format_args!("Unable to fetch novel: {error}"));
                return ExitCode::FAILURE
            }
        }
    };

    stdout.write_fmtn(format_args!("{:#?}", info));
    ExitCode::SUCCESS
}

fn run(stdio: Stdio, args: cli::Cli) -> ExitCode {
    if args.novel.kind().is_syosetu() {
        run_syosetu(stdio, args)
    } else {
        run_kakuyomu(stdio, args)
    }
}

fn main() -> ExitCode {
    let stdio = Stdio::new();

    match cli::Cli::new() {
        Some(Ok(args)) => run(stdio, args),
        Some(Err(code)) => code,
        None => match args_from_stdin(&stdio) {
            Ok(args) => run(stdio, args),
            Err(code) => code,
        }
    }
}
