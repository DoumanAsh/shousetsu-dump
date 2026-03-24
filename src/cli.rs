use arg::Args;

use std::env;
use std::process::ExitCode;
use core::num::NonZeroUsize;

use shousetsu_dump::novel;

pub fn default_from_value() -> NonZeroUsize {
    unsafe {
        core::num::NonZeroUsize::new_unchecked(1)
    }
}

#[derive(Args, Debug)]
#[arg(infer_name)]
///Utility to download text of the web novels
///
///Supported websites:
///- kakuyomu.jp
///- syosetu.com
///
///In case of issue please file issue on https://github.com/DoumanAsh/shousetsu-dump
///
///Always include URL to the novel alongside command line parameters you run
pub struct Cli {
    #[arg(long, default_value = "default_from_value()")]
    ///Specify from which chapter to start dumping. Default: 1.
    pub from: NonZeroUsize,
    #[arg(long)]
    ///Specify until which chapter to dump.
    pub to: Option<NonZeroUsize>,
    #[arg(long, short)]
    ///Output file name. By default writes ./<title>.md
    pub out: Option<String>,
    #[arg(required)]
    ///Id of the novel to dump (e.g. kakuyomu.jp/works/1177354054935164320, novel18.syosetu.com/n9598df/)
    pub novel: novel::Id,
    #[arg(long, default_value = "0")]
    ///Number of chapters to download at most per interval. Defaults to no limit.
    pub rate: u16,
    #[arg(long, default_value = "1")]
    ///Interval between rated downloads. Defaults to 1 second.
    pub rate_interval: u64,
    #[arg(short, long, default_value = "false")]
    ///Specifies to fetch novel index without download. Defaults false.
    pub dry: bool,
}


impl Cli {
    #[inline]
    pub fn new() -> Option<Result<Self, ExitCode>> {
        let args: Vec<_> = env::args().skip(1).collect();

        if args.is_empty() {
            return None;
        }

        match Self::from_args(args.iter().map(String::as_str)) {
            Ok(args) => Some(Ok(args)),
            Err(arg::ParseKind::Sub(name, arg::ParseError::HelpRequested(help))) => {
                println!("{name}: {}", help);
                Some(Err(ExitCode::SUCCESS))
            },
            Err(arg::ParseKind::Top(arg::ParseError::HelpRequested(help))) => {
                println!("{}", help);
                Some(Err(ExitCode::SUCCESS))
            },
            Err(error) => {
                eprintln!("{}", error);
                Some(Err(ExitCode::FAILURE))
            }
        }
    }
}
