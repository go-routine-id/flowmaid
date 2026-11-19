//! probe
use std::cell::Cell;
/// default
pub const BLOCKS_DEFAULT: bool = cfg!(flowmaid_blocks);
thread_local! { static OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) }; }
/// on?
pub fn blocks_on() -> bool { OVERRIDE.with(|c| c.get()).unwrap_or(BLOCKS_DEFAULT) }
/// scoped
pub fn with_blocks<T>(on: bool, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<bool>);
    impl Drop for Reset { fn drop(&mut self) { OVERRIDE.with(|c| c.set(self.0)); } }
    let _r = Reset(OVERRIDE.with(|c| c.replace(Some(on))));
    f()
}
#[cfg(test)]
mod tests { use super::*; #[test] fn t() { assert_eq!(blocks_on(), BLOCKS_DEFAULT); assert!(with_blocks(true, blocks_on)); assert_eq!(blocks_on(), BLOCKS_DEFAULT); } }
