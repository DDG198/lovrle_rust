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
