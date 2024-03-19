use std::ops::Range;

use proptest::{
    prelude::Arbitrary,
    strategy::{BoxedStrategy, Strategy},
};

use crate::road::RectangleOccupier;

impl Arbitrary for RectangleOccupier {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        return (
            -10_000isize..10_000,
            -10_000isize..10_000,
            1usize..10_000,
            1usize..10_000,
        )
            .prop_map(|(front, right, width, length)| Self {
                front,
                right,
                width,
                length,
            })
            .boxed();
    }

    type Strategy = BoxedStrategy<Self>;
}

pub fn arb_rectangle_occupier(
    front_range: Range<isize>,
    right_range: Range<isize>,
    width_max: usize,
    length_max: usize,
) -> impl Strategy<Value = RectangleOccupier> {
    (front_range, right_range, 1..width_max, 1..length_max).prop_map(
        |(front, right, width, length)| RectangleOccupier {
            front,
            right,
            width,
            length,
        },
    )
}
