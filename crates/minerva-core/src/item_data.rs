//! The Item Card's OSMS item files (ADR 0010), held apart from the map set in a
//! [`Loader`] of their own: what the Item Card shows, or what its alert says for want of
//! them.

use crate::item_lookup::ItemLookup;
use crate::loader::Loader;
use crate::map_index::SourceError;
use crate::osms_items::ItemSet;

impl Loader<ItemSet> {
    /// The items held, or what the alert says for want of them.
    pub fn items(&self) -> Result<&ItemSet, ItemLookup> {
        match (self.held(), self.failed()) {
            (Some(items), _) => Ok(items),
            (None, None) => Err(ItemLookup::ItemsLoading),
            (None, Some(SourceError::Unreachable(_))) => Err(ItemLookup::ItemsUnavailable),
            (None, Some(SourceError::FormatChanged(_))) => Err(ItemLookup::ItemsFormatChanged),
        }
    }
}
