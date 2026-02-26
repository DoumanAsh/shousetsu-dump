use std::borrow::Cow;
use core::fmt::{self, Write};

use crate::http;
use crate::utils::StrExt;

pub mod kakuyomu;
pub use kakuyomu::fetch_novel_info as fetch_kakuyomu;
pub mod syosetu;
pub use syosetu::fetch_novel_info as fetch_syosetu;

#[derive(Debug)]
pub enum Error {
    MissingIndex,
    MissingTitle,
    MissingChapters,
    InvalidKakuyomuChapters(serde_json::Error),
    Transient(http::Error),
    Internal(http::Error),
    NotFound,
    InvalidChapterContent(Cow<'static, str>),
}

impl Error {
    pub fn from_http(error: http::Error) -> Self {
        match &error {
            http::Error::Internal(_) => Self::Internal(error),
            _ => Self::Transient(error),
        }
    }

    #[cold]
    #[inline(never)]
    pub fn invalid_chapter_content(error: Cow<'static, str>) -> Self {
        Self::InvalidChapterContent(error)
    }
}

impl fmt::Display for Error {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIndex => fmt.write_str("Unable to find novel's index"),
            Self::MissingTitle => fmt.write_str("Unable to find novel's title"),
            Self::MissingChapters => fmt.write_str("Unable to find novel's chapter"),
            Self::InvalidKakuyomuChapters(error) => fmt.write_fmt(format_args!("Failed to parse APOLLO_STATE: {error}")),
            Self::Transient(error) => fmt.write_fmt(format_args!("Unable to make request: {error}")),
            Self::Internal(error) => fmt.write_fmt(format_args!("Internal error: {error}")),
            Self::NotFound => fmt.write_str("No such novel found"),
            Self::InvalidChapterContent(error) => fmt.write_fmt(format_args!("Unable to parse chapter body: {error}")),
        }
    }
}

type Result<T, E=Error> = core::result::Result<T, E>;

//Limited buffer to hold maximum possible id
pub type IdBuffer = str_buf::StrBuf<{str_buf::capacity(20)}>;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BackendKind {
    ///https://kakuyomu.jp/works/<id>
    Kakuyomu,
    //Syosetu uses range from `N0000A` to `N9999Z`
    ///https://ncode.syosetu.com/<id>
    Syosetu,
    ///https://novel18.syosetu.com/<id>
    R18Syosetu
}

impl BackendKind {
    #[inline(always)]
    ///Returns whether kind belongs kakuyomu
    pub const fn is_kakuyomu(&self) -> bool {
        matches!(self, Self::Kakuyomu)
    }

    #[inline(always)]
    ///Returns whether kind belongs to either of syosetu novels
    pub const fn is_syosetu(&self) -> bool {
        matches!(self, Self::Syosetu | Self::R18Syosetu)
    }

    #[inline(always)]
    ///Returns whether it is syosetu r18 novel
    pub const fn is_syosetu_r18(&self) -> bool {
        matches!(self, Self::R18Syosetu)
    }

}

#[derive(Debug, Clone)]
///Novel ID
pub struct Id {
    kind: BackendKind,
    id: IdBuffer,
}

impl Id {
    ///Attempts to infer id from the string input
    pub fn try_parse(mut id: &str) -> Option<Self> {
        if let Some(stripped) = id.strip_suffix('/') {
            id = stripped;
        }

        let (kind, id) = if let Some([id, prefix]) = id.rsplit_exact_by::<2>('/') {
            let kind = if prefix.ends_with("ncode.syosetu.com") {
                BackendKind::Syosetu
            } else if prefix.ends_with("novel18.syosetu.com") {
                BackendKind::R18Syosetu
            } else if prefix.ends_with("kakuyomu.jp/works") {
                BackendKind::Kakuyomu
            } else {
                return None;
            };
            (kind, id)
        } else {
            let kind = if id.starts_with('n') {
                //We cannot determine whether it is r18 or not so just assume syosetu always and automatically try to use API to fetch both
                BackendKind::Syosetu
            } else {
                BackendKind::Kakuyomu
            };
            (kind, id)
        };

        IdBuffer::from_str_checked(id).map(|id| Self {
            kind,
            id,
        }).ok()
    }

    #[inline(always)]
    pub const fn kind(&self) -> BackendKind {
        self.kind
    }

    #[inline(always)]
    pub const fn id(&self) -> &str {
        self.id.as_str()
    }

    #[inline(always)]
    pub const fn url(&self) -> Url<'_> {
        Url(self)
    }
}

impl core::str::FromStr for Id {
    type Err = ();

    #[inline(always)]
    fn from_str(id: &str) -> Result<Self, Self::Err> {
        match Self::try_parse(id) {
            Some(id) => Ok(id),
            None => Err(())
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Url<'a>(&'a Id);

impl Url<'_> {
    #[inline(always)]
    pub fn write_to(&self, out: &mut impl fmt::Write) -> fmt::Result {
        let Id { kind, id } = self.0;
        match kind {
            BackendKind::Syosetu => write!(out, "https://api.syosetu.com/novelapi/api/?out=json&ncode={id}"),
            BackendKind::R18Syosetu => write!(out, "https://api.syosetu.com/novel18api/api/?out=json&ncode={id}"),
            BackendKind::Kakuyomu => write!(out, "https://kakuyomu.jp/works/{id}"),
        }
    }
}

impl fmt::Debug for Url<'_> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(fmt)
    }
}

impl fmt::Display for Url<'_> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(fmt)
    }
}

pub struct Title<'a> {
    pub name: &'a str,
    pub author: Option<&'a str>,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Chapter {
    id: IdBuffer,
}

impl Chapter {
    ///Generates URL to fetch chapter
    pub fn preapre_url(&self, id: &Id, out: &mut String) {
        let novel_id = id.id.as_str();
        let chapter_id = self.id.as_str();
        let _ = match id.kind() {
            BackendKind::Syosetu => write!(out, "https://ncode.syosetu.com/{novel_id}/{chapter_id}"),
            BackendKind::R18Syosetu => write!(out, "https://novel18.syosetu.com/{novel_id}/{chapter_id}"),
            BackendKind::Kakuyomu => write!(out, "https://kakuyomu.jp/works/{novel_id}/episodes/{chapter_id}"),
        };
    }
}

///Parsed content of the chapter
pub trait ChapterContent {
    ///Extracts Chapter title
    fn title(&self) -> Result<String>;
    ///Extracts Chapter content
    fn lines(&self) -> Result<impl Iterator<Item = Line> + '_>;
}

///Possible variants for novel's body line
pub enum Line {
    ///Line of text to write
    Paragraph(String),
    ///URL with image and alt title
    Img(String, String),
    ///Indicates empty line/line break
    Break,
}

///Describes novel information
pub trait NovelInfo: fmt::Debug {
    type ChapterIter: ExactSizeIterator<Item = Chapter>;
    ///Returns novel id
    fn id(&self) -> &Id;
    ///Returns title of the novel
    fn title(&self) -> Result<Title<'_>, Error>;
    ///Returns iterator of chapters
    fn chapters(&self) -> Result<Self::ChapterIter, Error>;

    ///Retrieves chapter content extractor
    fn extract_chapter_content<'a>(&'a self, body: &'a str) -> impl ChapterContent + 'a;

    #[inline(always)]
    ///Returns list of headers to use when fetching chapter data
    fn headers(&self) -> &[(&str, &str)] {
        &[]
    }
}

pub trait GetNovelInfo {
    fn get_novel(&self, id: Id, http: &http::Client) -> Result<impl NovelInfo>;
}

impl<N: NovelInfo, T: Fn(Id, &http::Client) -> Result<N>> GetNovelInfo for T {
    #[inline(always)]
    fn get_novel(&self, id: Id, http: &http::Client) -> Result<impl NovelInfo> {
        (self)(id, http)
    }
}
