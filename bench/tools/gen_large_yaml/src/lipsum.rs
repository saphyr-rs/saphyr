// Vendored from lipsum-0.9.1 due to failure to build with newer rust versions.

//! Lorem ipsum generator.
//!
//! This crate generates pseudo-Latin [lorem ipsum placeholder
//! text][wiki]. The traditional lorem ipsum text start like this:
//!
//! > Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do
//! > eiusmod tempor incididunt ut labore et dolore magna aliqua.
//!
//! This text is in the [`LOREM_IPSUM`] constant. Random looking text
//! like the above can be generated using the [`lipsum`] function. The
//! function allows you to generate as much text as desired and each
//! invocation will generate different text.
//!
//! The random looking text is generated using a [Markov chain] of
//! order two, which simply means that the next word is based on the
//! previous two words in the input texts. The Markov chain can be
//! used with other input texts by creating an instance of
//! [`MarkovChain`] and calling its [`learn`] method.
//!
//! [wiki]: https://en.wikipedia.org/wiki/Lorem_ipsum
//! [`lipsum`]: fn.lipsum.html
//! [`MarkovChain`]: struct.MarkovChain.html
//! [`learn`]: struct.MarkovChain.html#method.learn
//! [Markov chain]: https://en.wikipedia.org/wiki/Markov_chain

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use rand::prelude::IndexedRandom;
use rand::{Rng, RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::collections::HashMap;

/// A bigram is simply two consecutive words.
pub type Bigram<'a> = (&'a str, &'a str);

/// Simple order two Markov chain implementation.
///
/// The [Markov chain] is a chain of order two, which means that it
/// will use the previous two words (a bigram) when predicting the
/// next word. This is normally enough to generate random text that
/// looks somewhat plausible. The implementation is based on
/// [Generating arbitrary text with Markov chains in Rust][blog post].
///
/// [Markov chain]: https://en.wikipedia.org/wiki/Markov_chain
/// [blog post]: https://blakewilliams.me/posts/generating-arbitrary-text-with-markov-chains-in-rust
#[derive(Debug, Clone, Default)]
pub struct MarkovChain<'a> {
    map: HashMap<Bigram<'a>, Vec<&'a str>>,
    keys: Vec<Bigram<'a>>,
}

impl<'a> MarkovChain<'a> {
    /// Create a new empty Markov chain.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() {
    /// use lipsum::MarkovChain;
    /// use rand::SeedableRng;
    /// use rand_chacha::ChaCha20Rng;
    ///
    /// let mut chain = MarkovChain::new();
    /// chain.learn("infra-red red orange yellow green blue indigo x-ray");
    ///
    /// let mut rng = ChaCha20Rng::seed_from_u64(0);
    ///
    /// // The chain jumps consistently like this:
    /// assert_eq!(chain.generate_with_rng(&mut rng, 1), "Orange.");
    /// assert_eq!(chain.generate_with_rng(&mut rng, 1), "Infra-red.");
    /// assert_eq!(chain.generate_with_rng(&mut rng, 1), "Yellow.");
    /// # }
    /// ```
    pub fn new() -> MarkovChain<'a> {
        Default::default()
    }

    /// Add new text to the Markov chain. This can be called several
    /// times to build up the chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use lipsum::MarkovChain;
    ///
    /// let mut chain = MarkovChain::new();
    /// chain.learn("red green blue");
    /// assert_eq!(chain.words(("red", "green")), Some(&vec!["blue"]));
    ///
    /// chain.learn("red green yellow");
    /// assert_eq!(chain.words(("red", "green")), Some(&vec!["blue", "yellow"]));
    /// ```
    pub fn learn(&mut self, sentence: &'a str) {
        let words = sentence.split_whitespace().collect::<Vec<&str>>();
        for window in words.windows(3) {
            let (a, b, c) = (window[0], window[1], window[2]);
            self.map.entry((a, b)).or_default().push(c);
        }
        // Sync the keys with the current map.
        self.keys = self.map.keys().cloned().collect();
        self.keys.sort_unstable();
    }

    /// Returs the number of states in the Markov chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use lipsum::MarkovChain;
    ///
    /// let mut chain = MarkovChain::new();
    /// assert_eq!(chain.len(), 0);
    ///
    /// chain.learn("red orange yellow green blue indigo");
    /// assert_eq!(chain.len(), 4);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the Markov chain has no states.
    ///
    /// # Examples
    ///
    /// ```
    /// use lipsum::MarkovChain;
    ///
    /// let mut chain = MarkovChain::new();
    /// assert!(chain.is_empty());
    ///
    /// chain.learn("foo bar baz");
    /// assert!(!chain.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the possible words following the given bigram, or `None`
    /// if the state is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use lipsum::MarkovChain;
    ///
    /// let mut chain = MarkovChain::new();
    /// chain.learn("red green blue");
    /// assert_eq!(chain.words(("red", "green")), Some(&vec!["blue"]));
    /// assert_eq!(chain.words(("foo", "bar")), None);
    /// ```
    pub fn words(&self, state: Bigram<'a>) -> Option<&Vec<&str>> {
        self.map.get(&state)
    }

    /// Generate a sentence with `n` words of lorem ipsum text. The
    /// sentence will start from a random point in the Markov chain
    /// generated using the specified random number generator,
    /// and a `.` will be added as necessary to form a full sentence.
    ///
    /// See [`generate_with_rng_from`] if you want to control the
    /// starting point for the generated text and see [`iter_with_rng`]
    /// if you simply want a sequence of words.
    ///
    /// # Examples
    ///
    /// Generating the sounds of a grandfather clock:
    ///
    /// ```
    /// use lipsum::MarkovChain;
    /// use rand_chacha::ChaCha20Rng;
    /// use rand::SeedableRng;
    ///
    /// let mut chain = MarkovChain::new();
    /// chain.learn("Tick, Tock, Tick, Tock, Ding! Tick, Tock, Ding! Ding!");
    /// println!("{}", chain.generate_with_rng(ChaCha20Rng::seed_from_u64(0), 15));
    /// ```
    ///
    /// The output looks like this:
    ///
    /// > Ding! Tick, Tock, Tick, Tock, Ding! Ding! Tock, Ding! Tick,
    /// > Tock, Tick, Tock, Tick, Tock.
    ///
    /// [`generate_with_rng_from`]: struct.MarkovChain.html#method.generate_with_rng_from
    /// [`iter_with_rng`]: struct.MarkovChain.html#method.iter_with_rng
    pub fn generate_with_rng<R: Rng>(&self, rng: R, n: usize) -> String {
        join_words(self.iter_with_rng(rng).take(n))
    }

    /// Generate a sentence with `n` words of lorem ipsum text. The sentence
    /// will start from a predetermined point in the Markov chain generated
    /// using the default random number generator and a `.` will be added as
    /// necessary to form a full sentence.
    ///
    /// See [`generate_from`] if you want to control the starting point for the
    /// generated text and see [`iter`] if you simply want a sequence of words.
    ///
    /// # Examples
    ///
    /// Generating the sounds of a grandfather clock:
    ///
    /// ```
    /// use lipsum::MarkovChain;
    ///
    /// let mut chain = MarkovChain::new();
    /// chain.learn("Tick, Tock, Tick, Tock, Ding! Tick, Tock, Ding! Ding!");
    /// println!("{}", chain.generate(15));
    /// ```
    ///
    /// The output looks like this:
    ///
    /// > Ding! Tick, Tock, Tick, Tock, Ding! Ding! Tock, Ding! Tick,
    /// > Tock, Tick, Tock, Tick, Tock.
    ///
    /// [`generate_from`]: struct.MarkovChain.html#method.generate_from
    /// [`iter`]: struct.MarkovChain.html#method.iter
    pub fn generate(&self, n: usize) -> String {
        self.generate_with_rng(default_rng(), n)
    }

    /// Generate a sentence with `n` words of lorem ipsum text. The
    /// sentence will start from the given bigram and a `.` will be
    /// added as necessary to form a full sentence.
    ///
    /// Use [`generate_with_rng`] if the starting point is not important. See
    /// [`iter_with_rng_from`] if you want a sequence of words that you can
    /// format yourself.
    ///
    /// [`generate_with_rng`]: struct.MarkovChain.html#method.generate_with_rng
    /// [`iter_with_rng_from`]: struct.MarkovChain.html#method.iter_with_rng_from
    pub fn generate_with_rng_from<R: Rng>(&self, rng: R, n: usize, from: Bigram<'a>) -> String {
        join_words(self.iter_with_rng_from(rng, from).take(n))
    }

    /// Generate a sentence with `n` words of lorem ipsum text. The
    /// sentence will start from the given bigram and a `.` will be
    /// added as necessary to form a full sentence.
    ///
    /// Use [`generate`] if the starting point is not important. See
    /// [`iter_from`] if you want a sequence of words that you can
    /// format yourself.
    ///
    /// [`generate`]: struct.MarkovChain.html#method.generate
    /// [`iter_from`]: struct.MarkovChain.html#method.iter_from
    pub fn generate_from(&self, n: usize, from: Bigram<'a>) -> String {
        self.generate_with_rng_from(default_rng(), n, from)
    }

    /// Make a never-ending iterator over the words in the Markov
    /// chain. The iterator starts at a random point in the chain.
    pub fn iter_with_rng<R: Rng>(&self, mut rng: R) -> Words<'_, R> {
        let initial_bigram = if self.is_empty() {
            ("", "")
        } else {
            *self.keys.choose(&mut rng).unwrap()
        };
        self.iter_with_rng_from(rng, initial_bigram)
    }

    /// Make a never-ending iterator over the words in the Markov chain. The
    /// iterator starts at a predetermined point in the chain.
    pub fn iter(&self) -> Words<'_, impl Rng> {
        self.iter_with_rng(default_rng())
    }

    /// Make a never-ending iterator over the words in the Markov
    /// chain. The iterator starts at the given bigram.
    pub fn iter_with_rng_from<R: Rng>(&self, rng: R, from: Bigram<'a>) -> Words<'_, R> {
        Words {
            map: &self.map,
            rng,
            keys: &self.keys,
            state: from,
        }
    }

    /// Make a never-ending iterator over the words in the Markov
    /// chain. The iterator starts at the given bigram.
    pub fn iter_from(&self, from: Bigram<'a>) -> Words<'_, impl Rng> {
        self.iter_with_rng_from(default_rng(), from)
    }
}

/// Provide a default random number generator. This generator is seeded and will
/// always produce the same sequence of numbers. The seed is chosen to yield
/// good results for the included Markov chain.
fn default_rng() -> impl Rng {
    ChaCha20Rng::seed_from_u64(97)
}

/// Never-ending iterator over words in the Markov chain.
///
/// Generated with the [`iter`] or [`iter_from`] methods.
///
/// [`iter`]: struct.MarkovChain.html#method.iter
/// [`iter_from`]: struct.MarkovChain.html#method.iter_from
pub struct Words<'a, R: Rng> {
    map: &'a HashMap<Bigram<'a>, Vec<&'a str>>,
    rng: R,
    keys: &'a Vec<Bigram<'a>>,
    state: Bigram<'a>,
}

impl<'a, R: Rng> Iterator for Words<'a, R> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.map.is_empty() {
            return None;
        }

        let result = Some(self.state.0);

        while !self.map.contains_key(&self.state) {
            self.state = *self.keys.choose(&mut self.rng).unwrap();
        }
        let next_words = &self.map[&self.state];
        let next = next_words.choose(&mut self.rng).unwrap();
        self.state = (self.state.1, next);
        result
    }
}

/// Check if `c` is an ASCII punctuation character.
fn is_ascii_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
}

/// Capitalize the first character in a string.
fn capitalize(word: &str) -> String {
    let idx = match word.chars().next() {
        Some(c) => c.len_utf8(),
        None => 0,
    };

    let mut result = String::with_capacity(word.len());
    result.push_str(&word[..idx].to_uppercase());
    result.push_str(&word[idx..]);
    result
}

/// Join words from an iterator. The first word is always capitalized
/// and the generated sentence will end with `'.'` if it doesn't
/// already end with some other ASCII punctuation character.
fn join_words<'a, I: Iterator<Item = &'a str>>(mut words: I) -> String {
    match words.next() {
        None => String::new(),
        Some(word) => {
            // Punctuation characters which ends a sentence.
            let punctuation: &[char] = &['.', '!', '?'];

            let mut sentence = capitalize(word);
            let mut needs_cap = sentence.ends_with(punctuation);

            // Add remaining words.
            for word in words {
                sentence.push(' ');

                if needs_cap {
                    sentence.push_str(&capitalize(word));
                } else {
                    sentence.push_str(word);
                }

                needs_cap = word.ends_with(punctuation);
            }

            // Ensure the sentence ends with either one of ".!?".
            if !sentence.ends_with(punctuation) {
                // Trim all trailing punctuation characters to avoid
                // adding '.' after a ',' or similar.
                let idx = sentence.trim_end_matches(is_ascii_punctuation).len();
                sentence.truncate(idx);
                sentence.push('.');
            }

            sentence
        }
    }
}

/// The traditional lorem ipsum text as given in [Wikipedia]. Using
/// this text alone for a Markov chain of order two doesn't work very
/// well since each bigram (two consequtive words) is followed by just
/// one other word. In other words, the Markov chain will always
/// produce the same output and recreate the lorem ipsum text
/// precisely. However, combining it with the full text in
/// [`LIBER_PRIMUS`] works well.
///
/// [Wikipedia]: https://en.wikipedia.org/wiki/Lorem_ipsum
/// [`LIBER_PRIMUS`]: constant.LIBER_PRIMUS.html
pub const LOREM_IPSUM: &str = include_str!("lorem-ipsum.txt");

/// The first book in Cicero's work De finibus bonorum et malorum ("On
/// the ends of good and evil"). The lorem ipsum text in
/// [`LOREM_IPSUM`] is derived from part of this text.
///
/// [`LOREM_IPSUM`]: constant.LOREM_IPSUM.html
pub const LIBER_PRIMUS: &str = include_str!("liber-primus.txt");

thread_local! {
    // Markov chain generating lorem ipsum text.
    static LOREM_IPSUM_CHAIN: MarkovChain<'static> = {
        let mut chain = MarkovChain::new();
        // The cost of learning increases as more and more text is
        // added, so we start with the smallest text.
        chain.learn(LOREM_IPSUM);
        chain.learn(LIBER_PRIMUS);
        chain
    }
}

/// Generate `n` words of lorem ipsum text. The output will always start with
/// "Lorem ipsum".
///
/// The text continues with the standard lorem ipsum text from [`LOREM_IPSUM`]
/// and becomes randomly generated but deterministic if more than 18 words is
/// requested. See [`lipsum_words`] if fully random text is needed.
///
/// # Examples
///
/// ```
/// use lipsum::lipsum;
///
/// assert_eq!(lipsum(7), "Lorem ipsum dolor sit amet, consectetur adipiscing.");
/// ```
///
/// [`LOREM_IPSUM`]: constant.LOREM_IPSUM.html
/// [`lipsum_words`]: fn.lipsum_words.html
pub fn lipsum(n: usize) -> String {
    LOREM_IPSUM_CHAIN.with(|chain| chain.generate_from(n, ("Lorem", "ipsum")))
}

/// Generate `n` words of lorem ipsum text with a custom RNG. The output will
/// always start with "Lorem ipsum".
///
/// A custom RNG allows to base the markov chain on a different random number
/// sequence. This also allows using a regular [`thread_rng`] random number
/// generator. If that generator is used, the text will differ in each
/// invocation.
///
/// # Examples
///
/// ```
/// use lipsum::lipsum_with_rng;
/// use rand::thread_rng;
///
/// println!("{}", lipsum_with_rng(thread_rng(), 23));
/// // -> "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do
/// //     eiusmod tempor incididunt ut labore et dolore magnam aliquam
/// //     quaerat voluptatem. Ut enim."
/// ```
///
/// [`thread_rng`]: https://docs.rs/rand/latest/rand/fn.thread_rng.html
pub fn lipsum_with_rng(rng: impl Rng, n: usize) -> String {
    LOREM_IPSUM_CHAIN.with(|chain| chain.generate_with_rng_from(rng, n, ("Lorem", "ipsum")))
}

/// Generate `n` words of lorem ipsum text.
///
/// The text is deterministically sampled from a Markov chain based on
/// [`LOREM_IPSUM`]. Multiple sentences may be generated, depending on the
/// punctuation of the words being selected.
///
/// # Examples
///
/// ```
/// use lipsum::lipsum_words;
///
/// assert_eq!(lipsum_words(6), "Ullus investigandi veri, nisi inveneris, et.");
/// ```
///
/// [`LOREM_IPSUM`]: constant.LOREM_IPSUM.html
pub fn lipsum_words(n: usize) -> String {
    LOREM_IPSUM_CHAIN.with(|chain| chain.generate(n))
}

/// Generate `n` words of lorem ipsum text with a custom RNG.
///
/// A custom RNG allows to base the markov chain on a different random number
/// sequence. This also allows using a regular [`thread_rng`] random number
/// generator. If that generator is used, the text will differ in each
/// invocation.
///
/// # Examples
///
/// ```
/// use lipsum::lipsum_words_with_rng;
/// use rand::thread_rng;
///
/// println!("{}", lipsum_words_with_rng(thread_rng(), 7));
/// // -> "Quot homines, tot sententiae; falli igitur possumus."
/// ```
///
/// [`thread_rng`]: https://docs.rs/rand/latest/rand/fn.thread_rng.html
pub fn lipsum_words_with_rng(rng: impl Rng, n: usize) -> String {
    LOREM_IPSUM_CHAIN.with(|chain| chain.generate_with_rng(rng, n))
}

/// Minimum number of words to include in a title.
const TITLE_MIN_WORDS: usize = 3;
/// Maximum number of words to include in a title.
const TITLE_MAX_WORDS: usize = 8;
/// Words shorter than this size are not capitalized.
const TITLE_SMALL_WORD: usize = 3;

/// Generate a short lorem ipsum text with words in title case.
///
/// The words are capitalized and stripped for punctuation characters.
///
/// # Examples
///
/// ```
/// use lipsum::lipsum_title;
///
/// println!("{}", lipsum_title());
/// ```
///
/// This will generate a string like
///
/// > Grate Meminit et Praesentibus
///
/// which should be suitable for use in a document title for section
/// heading.
pub fn lipsum_title() -> String {
    lipsum_title_with_rng(default_rng())
}

/// Generate a short lorem ipsum text with words in title case with a custom RNG.
///
/// A custom RNG allows to base the markov chain on a different random number
/// sequence. This also allows using a regular [`thread_rng`] random number
/// generator. If that generator is used, the text will differ in each
/// invocation.
///
/// The words are capitalized and stripped for punctuation characters.
///
/// # Examples
///
/// ```
/// use lipsum::lipsum_title_with_rng;
/// use rand::thread_rng;
///
/// println!("{}", lipsum_title_with_rng(thread_rng()));
/// ```
///
/// This will generate a string like
///
/// > Grate Meminit et Praesentibus
///
/// which should be suitable for use in a document title for section
/// heading.
///
/// [`thread_rng`]: https://docs.rs/rand/latest/rand/fn.thread_rng.html
pub fn lipsum_title_with_rng(mut rng: impl Rng) -> String {
    LOREM_IPSUM_CHAIN.with(|chain| {
        let n = rng.random_range(TITLE_MIN_WORDS..TITLE_MAX_WORDS);
        // The average word length with our corpus is 7.6 bytes so
        // this capacity will avoid most allocations.
        let mut title = String::with_capacity(8 * n);

        let words = chain
            .iter_with_rng(rng)
            .map(|word| word.trim_matches(is_ascii_punctuation))
            .filter(|word| !word.is_empty())
            .take(n);

        for (i, word) in words.enumerate() {
            if i > 0 {
                title.push(' ');
            }

            // Capitalize the first word and all long words.
            if i == 0 || word.len() > TITLE_SMALL_WORD {
                title.push_str(&capitalize(word));
            } else {
                title.push_str(word);
            }
        }
        title
    })
}
