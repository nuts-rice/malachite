use core::mem::size_of;

use malachitebft_app_channel::app::types::core::Round;
use malachitebft_test::Height;

pub type UndecidedValueKey = (HeightKey, RoundKey);

#[derive(Copy, Clone, Debug)]
pub struct HeightKey;

impl redb::Value for HeightKey {
    type SelfType<'a> = Height;
    type AsBytes<'a> = [u8; size_of::<u64>()];

    fn fixed_width() -> Option<usize> {
        Some(size_of::<u64>())
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let height = <u64 as redb::Value>::from_bytes(data);

        Height::new(height)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        <u64 as redb::Value>::as_bytes(&value.as_u64())
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Height")
    }
}

impl redb::Key for HeightKey {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        <u64 as redb::Key>::compare(data1, data2)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RoundKey;

impl redb::Value for RoundKey {
    type SelfType<'a> = Round;
    type AsBytes<'a> = [u8; size_of::<i64>()];

    fn fixed_width() -> Option<usize> {
        Some(size_of::<i64>())
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let round = <i64 as redb::Value>::from_bytes(data);
        Round::from(round)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        <i64 as redb::Value>::as_bytes(&value.as_i64())
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Round")
    }
}

impl redb::Key for RoundKey {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        <i64 as redb::Key>::compare(data1, data2)
    }
}
