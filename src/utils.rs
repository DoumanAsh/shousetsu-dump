use std::io;
use core::mem;

use crate::stdio;

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
    last_stop_time: std::time::Instant,
    rate_limit: u16,
    sleep_interval: core::time::Duration,
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

    pub fn on_chapter_finished(&mut self, stdout: &mut stdio::Out<impl io::Write + core::fmt::Debug, stdio::behavior::Ignore>) {
        if self.rate_limit == 0 {
            return;
        }

        self.count += 1;
        if self.count >= self.rate_limit {
            if let Some(sleep_time) = self.sleep_interval.checked_sub(self.last_stop_time.elapsed()) {
                stdout.write_fmtn(format_args!("Wait {:.3}s...", sleep_time.as_secs_f64()));
                std::thread::sleep(sleep_time);
            }
            self.count = 0;
            self.last_stop_time = std::time::Instant::now();
        }
    }
}
