use serde_derive::Deserialize;

use core::fmt::{self, Write};

use crate::http;
use super::{Id, BackendKind, Chapter, Line, IdBuffer};

pub struct ChapterIter {
    start: u32,
    end: u32,
}

impl Iterator for ChapterIter {
    type Item = Chapter;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let result = self.start;
            self.start = self.start.saturating_add(1);

            let mut id = IdBuffer::new();
            let _ = write!(&mut id, "{}", result);
            Some(Chapter {
                id
            })
        } else {
            None
        }
    }
}

impl ExactSizeIterator for ChapterIter {
    #[inline(always)]
    fn len(&self) -> usize {
        //start <= end so at least 1 element since range is inclusive
        self.end.saturating_sub(self.start).saturating_add(1) as _
    }
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    #[allow(unused)]
    #[serde(rename = "allcount")]
    count: usize
}

#[derive(Debug, Deserialize)]
pub struct Info {
    pub title: String,
    pub writer: String,
    #[serde(rename = "general_all_no")]
    pub chapter_count: u32,
    #[serde(rename = "novelupdated_at")]
    pub updated_at: str_buf::StrBuf<{str_buf::capacity(19)}>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ApiResponse {
    Meta(Meta),
    Info(Info)
}

pub struct NovelInfo {
    id: Id,
    novel: Info,
}

impl NovelInfo {
    #[inline(always)]
    const fn new(id: Id, novel: Info) -> Self {
        Self {
            id,
            novel
        }
    }
}

impl fmt::Debug for NovelInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Novel Info")
           .field("Id", &self.id.id())
           .field("Url", &self.id.url())
           .field("Title", &self.novel.title)
           .field("Author", &self.novel.writer)
           .field("Number of Chapter", &self.novel.chapter_count)
           .field("Updated at", &self.novel.updated_at)
           .finish()
    }
}

struct ChapterContent(scraper::Html);

impl super::ChapterContent for ChapterContent {
    fn title(&self) -> super::Result<String> {
        Err(super::Error::invalid_chapter_content("not implemented".into()))
    }

    fn lines(&self) -> super::Result<impl Iterator<Item = Line> + '_> {
        Err::<core::array::IntoIter<Line, 0>, _>(super::Error::invalid_chapter_content("not implemented".into()))
    }
}


impl super::NovelInfo for NovelInfo {
    type ChapterIter = ChapterIter;

    #[inline(always)]
    fn id(&self) -> &Id {
        &self.id
    }

    #[inline(always)]
    fn title(&self) -> super::Result<super::Title<'_>> {
        Ok(super::Title {
            name: self.novel.title.as_str(),
            author: Some(self.novel.writer.as_str()),
        })
    }
    #[inline(always)]
    fn chapters(&self) -> super::Result<Self::ChapterIter> {
        Ok(ChapterIter {
            start: 1,
            end: self.novel.chapter_count
        })
    }

    fn extract_chapter_content<'a>(&'a self, body: &'a str) -> impl super::ChapterContent + 'a {
        ChapterContent(scraper::Html::parse_document(body))
    }

    fn headers(&self) -> &[(&str, &str)] {
        if self.id.kind().is_syosetu_r18() {
            &[("Cookie", "over18=yes")]
        } else {
            &[]
        }
    }
}

pub fn fetch_novel_info(mut id: Id, http: &http::Client) -> super::Result<NovelInfo> {
    let mut url = String::new();
    let _ = id.url().write_to(&mut url);

    //API always returns array of allcount object + novel information, in this case query by ncode so there can only be one
    //Reference: https://dev.syosetu.com/man/api/#output
    let mut info: http::Json<Vec<ApiResponse>> = match http.get(&url) {
        Ok(resp) => resp,
        Err(error) => return Err(super::Error::from_http(error))
    };

    if let Some(ApiResponse::Info(novel)) = info.0.pop() {
        return Ok(NovelInfo::new(id, novel));
    }

    //novel not found, try second API
    id.kind = match id.kind() {
        BackendKind::Syosetu => BackendKind::R18Syosetu,
        BackendKind::R18Syosetu => BackendKind::Syosetu,
        _ => panic!("This is impossible path. If you see this message, it is bug in my program"),
    };

    url.clear();
    let _ = id.url().write_to(&mut url);

    info = match http.get(&url) {
        Ok(resp) => resp,
        Err(error) => return Err(super::Error::from_http(error))
    };

    if let Some(ApiResponse::Info(novel)) = info.0.pop() {
        Ok(NovelInfo::new(id, novel))
    } else {
        Err(super::Error::NotFound)
    }
}
