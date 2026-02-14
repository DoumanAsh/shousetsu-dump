use std::io;
use core::{time, fmt};

type Response = ureq::http::Response<ureq::Body>;

const USER_AGENT: &str = env_smart::env!("{CARGO_PKG_NAME}/{CARGO_PKG_VERSION}");

#[derive(Debug)]
pub enum Error {
    StatusFailed(u16),
    Transport(ureq::Error),
    Internal(ureq::Error),
    Read(io::Error)
}

impl fmt::Display for Error {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatusFailed(code) => fmt.write_fmt(format_args!("Request failed with status={code}")),
            Self::Transport(reason) => fmt.write_fmt(format_args!("Unable to fetch: {reason}")),
            Self::Internal(reason) => fmt.write_fmt(format_args!("Internal error: {reason}")),
            Self::Read(reason) => fmt.write_fmt(format_args!("Unable to read response: {reason}")),
        }
    }
}

impl From<ureq::Error> for Error {
    #[inline]
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::StatusCode(code) => Self::StatusFailed(code),
            error @ ureq::Error::Json(_) => Self::Internal(error),
            error => Self::Transport(error),
        }
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(value: io::Error) -> Self {
        Self::Read(value)
    }
}

pub trait FromResponse: Sized {
    fn read_response(resp: Response) -> Result<Self, Error>;
}

impl FromResponse for () {
    #[inline(always)]
    fn read_response(_: Response) -> Result<Self, Error> {
        Ok(())
    }
}

impl FromResponse for String {
    #[inline(always)]
    fn read_response(resp: Response) -> Result<Self, Error> {
        resp.into_body().read_to_string().map_err(Into::into)
    }
}

pub struct Json<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for Json<T> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, fmt)
    }
}

impl<T: serde::de::DeserializeOwned> FromResponse for Json<T> {
    #[inline(always)]
    fn read_response(resp: Response) -> Result<Self, Error> {
        const MAX_BODY_SIZE: u64 = 1024 * 1024;

        resp.into_body().with_config().limit(MAX_BODY_SIZE).read_json().map_err(Into::into).map(Self)
    }
}

pub struct Client {
    inner: ureq::Agent,
}

impl Client {
    #[inline]
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder().user_agent(USER_AGENT)
                                                  .proxy(ureq::Proxy::try_from_env())
                                                  .max_redirects(5)
                                                  .timeout_per_call(Some(time::Duration::from_secs(5)))
                                                  .timeout_connect(Some(time::Duration::from_secs(1)))
                                                  .build();
        Self {
            inner: ureq::Agent::new_with_config(config),
        }
    }

    pub fn get<T: FromResponse>(&self, url: &str) -> Result<T, Error> {
        let response = self.inner.get(url).call()?;
        if response.status() != 200 {
            Err(Error::StatusFailed(response.status().as_u16()))
        } else {
            T::read_response(response)
        }
    }
}
