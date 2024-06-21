use crate::{Result, Word};
use base_primitives::Log;

/// A collection of [`YlmEvent`]s.
///
/// [`YlmEvent`]: crate::YlmEvent
///
/// # Implementer's Guide
///
/// It should not be necessary to implement this trait manually. Instead, use
/// the [`ylm!`](crate::ylm!) procedural macro to parse Ylem syntax into
/// types that implement this trait.
pub trait YlmEventInterface: Sized {
    /// The name of this type.
    const NAME: &'static str;

    /// The number of variants.
    const COUNT: usize;

    /// Decode the events from the given log info.
    fn decode_raw_log(topics: &[Word], data: &[u8], validate: bool) -> Result<Self>;

    /// Decode the events from the given log object.
    fn decode_log(log: &Log, validate: bool) -> Result<Log<Self>> {
        Self::decode_raw_log(log.topics(), &log.data.data, validate)
            .map(|data| Log { address: log.address, data })
    }
}
