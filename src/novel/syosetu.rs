use scraper::html::Html;
use scraper::selector::Selector;
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
    selectors: ChapterSelector,
}

impl NovelInfo {
    #[inline(always)]
    fn new(id: Id, novel: Info) -> Self {
        Self {
            id,
            novel,
            selectors: ChapterSelector::new(),
        }
    }
}

impl fmt::Debug for NovelInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Novel Info")
           .field("Id", &self.id.id())
           .field("Url", &self.id.original_url())
           .field("Title", &self.novel.title)
           .field("Author", &self.novel.writer)
           .field("Number of Chapter", &self.novel.chapter_count)
           .field("Updated at", &self.novel.updated_at)
           .finish()
    }
}

pub struct ChapterSelector {
    body: Selector,
    title: Selector,
}

impl ChapterSelector {
    pub fn new() -> Self {
        Self {
            body: Selector::parse(".p-novel__text").unwrap(),
            title: Selector::parse(".p-novel__title").unwrap()
        }
    }
}


struct ChapterContent<'a> {
    html: Html,
    selectors: &'a ChapterSelector,
}

impl super::ChapterContent for ChapterContent<'_> {
    fn title(&self) -> super::Result<String> {
        match self.html.select(&self.selectors.title).next() {
            Some(title) => Ok(title.inner_html()),
            None => Err(super::Error::invalid_chapter_content("Cannot find .p-novel__title with title".into())),
        }
    }

    fn lines(&self) -> super::Result<impl Iterator<Item = Line> + '_> {
        let body = self.html.select(&self.selectors.body);

        Ok(body.map(|body| {
            [Line::ChapterDiv].into_iter().chain(body.child_elements().filter_map(|element| {
                if element.value().name() == "p" {
                    let text: String = element.text().collect();
                    let text = if text.trim().is_empty() {
                        None
                    } else {
                        Some(Line::Paragraph(text))
                    };

                    //Aggregate all descendent elements (except first which is current element)
                    //We look for `<br>` and `<img>` specifically
                    //Append text, if available at the end (this normally should not happen as single
                    //paragraph can hold only one type of element: text, line break, image
                    Some(element.descendent_elements().skip(1).filter_map(|child| {
                        let name = child.value().name();
                        if name == "br" {
                            Some(Line::Break)
                        } else if name == "img" {
                            if let Some(src) = child.attr("src") {
                                let alt = child.attr("alt").map(|alt| alt.to_owned()).unwrap_or_default();
                                let src = if src.starts_with("http") {
                                    src.to_owned()
                                } else {
                                    format!("https://{}", src.trim_start_matches('/'))
                                };
                                Some(Line::Img(src.to_owned(), alt))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }).chain(text))
                } else {
                    None
                }
            }).flatten()) //individual body
        }).flatten()) //body chain
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
        ChapterContent {
            html: Html::parse_document(body),
            selectors: &self.selectors
        }
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
