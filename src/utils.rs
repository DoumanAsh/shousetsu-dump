use std::time;
use core::{fmt, mem};

pub struct StringBuffer {
    buffer: String,
}

impl StringBuffer {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            buffer: String::new()
        }
    }

    #[inline(always)]
    pub fn acquire(&mut self) -> StringBufferGuard<'_> {
        StringBufferGuard {
            buffer: &mut self.buffer
        }
    }
}

pub struct StringBufferGuard<'a> {
    buffer: &'a mut String
}

impl core::ops::Deref for StringBufferGuard<'_> {
    type Target = String;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl core::ops::DerefMut for StringBufferGuard<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

impl fmt::Display for StringBufferGuard<'_> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.buffer, fmt)
    }
}

impl Drop for StringBufferGuard<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        self.buffer.clear();
    }
}

pub trait IterExt: Iterator {
    fn collect_exact<const N: usize>(self) -> Option<[Self::Item; N]>;
}

impl<T: Iterator> IterExt for T {
    fn collect_exact<const N: usize>(self) -> Option<[Self::Item; N]> {
        let mut result = [const { mem::MaybeUninit::uninit() }; N];

        let mut idx = 0;
        for part in self {
            *result.get_mut(idx)? = mem::MaybeUninit::new(part);
            idx += 1;
        }

        if idx < N {
            None
        } else {
            unsafe {
                Some(mem::transmute_copy(&result))
            }
        }
    }
}

pub trait StrExt {
    fn split_exact_by<const N: usize>(&self, ch: char) -> Option<[&str; N]>;
    fn rsplit_exact_by<const N: usize>(&self, ch: char) -> Option<[&str; N]>;
}

impl StrExt for str {
    #[inline(always)]
    fn split_exact_by<const N: usize>(&self, ch: char) -> Option<[&str; N]> {
        self.splitn(N, ch).collect_exact::<N>()
    }
    #[inline(always)]
    fn rsplit_exact_by<const N: usize>(&self, ch: char) -> Option<[&str; N]> {
        self.rsplitn(N, ch).collect_exact::<N>()
    }
}

pub struct PaceMaker {
    count: u16,
    last_stop_time: time::Instant,
    rate_limit: u16,
    sleep_interval: time::Duration,
}

impl PaceMaker {
    pub fn new(rate_limit: u16, sleep_interval: core::time::Duration) -> Self {
        Self {
            count: 0,
            last_stop_time: std::time::Instant::now(),
            rate_limit,
            sleep_interval,
        }
    }

    pub fn on_chapter_finished(&mut self) -> Option<Throttle<'_>> {
        if self.rate_limit == 0 {
            return None;
        }

        self.count += 1;
        if self.count >= self.rate_limit {
            Some(Throttle {
                duration: self.sleep_interval.checked_sub(self.last_stop_time.elapsed()),
                pace_maker: self,
            })
        } else {
            None
        }
    }
}

///Throttling result, returned when throttle may be necessary
///
///On drop resets `PaceMaker` and optionally perform sleep per `duration`
pub struct Throttle<'a> {
    pace_maker: &'a mut PaceMaker,
    duration: Option<time::Duration>
}

impl Throttle<'_> {
    #[inline(always)]
    pub fn duration(&self) -> Option<time::Duration> {
        self.duration
    }
}

impl Drop for Throttle<'_> {
    fn drop(&mut self) {
        self.pace_maker.count = 0;
        self.pace_maker.last_stop_time = time::Instant::now();
    }
}
