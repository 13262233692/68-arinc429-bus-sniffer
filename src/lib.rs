pub mod core;
pub mod decode;
pub mod io;
pub mod ui;
pub mod timing;
pub mod replay;

use timing::Timestamp;
use core::word::ArincWord;

#[derive(Debug, Clone)]
pub struct TimedWord {
    pub word: ArincWord,
    pub timestamp: Timestamp,
}

impl TimedWord {
    pub fn new(word: ArincWord, timestamp: Timestamp) -> Self {
        TimedWord { word, timestamp }
    }

    pub fn into_inner(self) -> (ArincWord, Timestamp) {
        (self.word, self.timestamp)
    }
}

impl std::ops::Deref for TimedWord {
    type Target = ArincWord;
    fn deref(&self) -> &Self::Target {
        &self.word
    }
}
