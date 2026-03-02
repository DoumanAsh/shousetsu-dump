//! Web novel dumping utilities

#![allow(clippy::style)]

mod cli;
use shousetsu_dump::stdio::Stdio;
use shousetsu_dump::{novel, http, utils};

use core::time;
use std::{fs, path, io};
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
        dry: false
    })
}

fn construct_file_path(dir: &str, name: &str) -> path::PathBuf {
    let mut path = path::PathBuf::from(dir);
    path.push(name);
    path.set_extension("md");

    path
}

fn perform_novel_fetch<N: novel::GetNovelInfo>(stdio: Stdio, args: cli::Cli, fetcher: N) -> ExitCode {
    use novel::NovelInfo;

    let http = http::Client::new();
    let mut stderr = stdio.stderr().ignore_errors();
    let mut stdout = stdio.stdout().ignore_errors();

    stdout.write_fmt(format_args!("{}: Fetch novel index...", args.novel.url()));
    let info = loop {
        match fetcher.get_novel(args.novel.clone(), &http) {
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

    //Verify we can extract information about novel
    let title = match info.title() {
        Ok(title) => title,
        Err(error) => {
            stdout.write_fmtn(format_args!("ERR"));
            stderr.write_fmtn(format_args!("{error}"));
            return ExitCode::FAILURE
        }
    };

    let chapters = match info.chapters() {
        Ok(chapters) => chapters,
        Err(error) => {
            stdout.write_fmtn(format_args!("ERR"));
            stderr.write_fmtn(format_args!("{error}"));
            return ExitCode::FAILURE;
        }
    };

    stdout.write_fmtn(format_args!("OK"));
    if args.dry {
        stdout.write_fmtn(format_args!("{:#?}", info));
        return ExitCode::SUCCESS
    }

    //Unless user specifies output file name, we shall use novel id to ensure it is always safe to write such file
    let novel_file_name = match args.out {
        Some(out) => path::PathBuf::from(out),
        None => construct_file_path(".", info.id().id()),
    };

    stdout.write_fmtn(format_args!("Number of chapters: {}", chapters.len()));
    let chapter_start_from = args.from.get();
    let chapter_until = match args.to {
        Some(max) => if max.get() > chapters.len() {
            stderr.write_fmtn(format_args!("Novel has only {} chapters, but option -to is set to '{}'", chapters.len(), max));
            return ExitCode::FAILURE
        } else {
            max.get()
        },
        None => chapters.len()
    };

    let mut novel_out = match fs::OpenOptions::new().create(true).write(true).truncate(true).open(&novel_file_name) {
        Ok(novel_out) => novel_out,
        Err(error) => {
            stderr.write_fmtn(format_args!("{}: Cannot write: {error}", novel_file_name.display()));
            return ExitCode::FAILURE
        }
    };

    macro_rules! write_novel {
        ($($arg:tt)*) => {
            if let Err(error) = std::io::Write::write_fmt(&mut novel_out, format_args!($($arg)*)) {
                stderr.write_fmtn(format_args!("{}: Cannot write: {error}", novel_file_name.display()));
                return ExitCode::FAILURE
            }
        };
    }

    write_novel!("{}\n===================\n", title.name);
    write_novel!("Original: {}\n\n", args.novel.original_url());

    let http_headers = info.headers();
    let mut pacer = utils::PaceMaker::new(args.rate, time::Duration::from_secs(args.rate_interval));
    let mut chapter_url = utils::StringBuffer::new();

    stdout.write_fmtn(format_args!("Download chapters: {}..{}", chapter_start_from, chapter_until));
    for (mut idx, chapter) in chapters.enumerate().skip(chapter_start_from.saturating_sub(1)).take(chapter_until.saturating_sub(chapter_start_from).saturating_add(1)) {
        use novel::ChapterContent;

        idx = idx.saturating_add(1);
        let mut chapter_url = chapter_url.acquire();
        chapter.preapre_url(info.id(), &mut chapter_url);

        stdout.write_fmt(format_args!(">>>{chapter_url}: Downloading..."));

        let body: String = match http.get_with_headers(&chapter_url, http_headers) {
            Ok(body) => body,
            Err(error) => {
                stdout.write_fmt(format_args!("ERR"));
                stderr.write_fmtn(format_args!("{error}"));
                continue
            }
        };

        //Verify full content can be extracted before writing
        let body = info.extract_chapter_content(&body);
        let title = match body.title() {
            Ok(title) => title,
            Err(error) => {
                stdout.write_fmtn(format_args!("ERR"));
                stderr.write_fmtn(format_args!("!!!Cannot extract title: {error}"));
                return ExitCode::FAILURE;
            }
        };
        let lines = match body.lines() {
            Ok(body) => body,
            Err(error) => {
                stdout.write_fmtn(format_args!("ERR"));
                stderr.write_fmtn(format_args!("!!!Cannot extract lines: {error}"));
                return ExitCode::FAILURE;
            }
        };

        stdout.write_fmtn(format_args!("OK"));
        write_novel!("\n{idx} {title}\n-------------------\n");
        let mut chapter_parted = false;
        for line in lines {
            match line {
                novel::Line::Break => write_novel!("<br/>\n\n"),
                novel::Line::Paragraph(line) => write_novel!("{line}\n\n"),
                novel::Line::Img(url, alt) => {
                    let url = http.resolve_url_location(&url);
                    write_novel!("![{alt}]({url})\n\n")
                },
                novel::Line::ChapterDiv => if chapter_parted {
                    write_novel!("\n<div class=\"ch_div\">-------</div>\n\n");
                } else {
                    chapter_parted = true;
                }
            }
        }

        if let Some(throttle) = pacer.on_chapter_finished() {
            if let Some(sleep_time) = throttle.duration() {
                stdout.write_fmtn(format_args!("Wait {:.3}s...", sleep_time.as_secs_f64()));
                std::thread::sleep(sleep_time);
            }
        }
    }

    if let Err(error) = io::Write::flush(&mut novel_out) {
        stderr.write_fmtn(format_args!("{}: Cannot write: {error}", novel_file_name.display()));
        return ExitCode::FAILURE
    }

    stdout.write_fmtn(format_args!("-------------------"));
    stdout.write_fmtn(format_args!("Output: {}", novel_file_name.display()));

    match novel_file_name.file_stem().and_then(|file_name| file_name.to_str()).or_else(|| novel_file_name.to_str()) {
        Some(file_name) => {
            stdout.write_fmtn(format_args!("Pandoc command to generate EPUB:\npandoc --metadata title=\"{title}\" --embed-resources --standalone --shift-heading-level-by=-1 --from=gfm -o \"{file_name}.epub\" \"{file_name}.md\"", title=title.name));
        },
        None => {
            stderr.write_fmtn(format_args!("Cannot format output file name into string, no pandoc command generated"));
        }
    }
    ExitCode::SUCCESS
}

fn run(stdio: Stdio, args: cli::Cli) -> ExitCode {
    if args.novel.kind().is_syosetu() {
        perform_novel_fetch(stdio, args, novel::fetch_syosetu)
    } else {
        perform_novel_fetch(stdio, args, novel::fetch_kakuyomu)
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
