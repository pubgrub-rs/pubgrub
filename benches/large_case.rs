#![feature(test)]
extern crate test;
use test::Bencher;

use pubgrub::range::Range;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::NumberVersion;

#[bench]
/// This is an entirely synthetic benchmark. It may not be realistic.
/// It is too slow to be useful in the long term. But hopefully that can be fixed by making `run` faster.
/// It has not bean minimized. There are meny `add_dependencies` that have no impact on the runtime or output.
fn large_case(b: &mut Bencher) {
    let mut dependency_provider = OfflineDependencyProvider::<u16, NumberVersion>::new();
    dependency_provider.add_dependencies(662, 19, vec![]);
    dependency_provider.add_dependencies(662, 18, vec![]);
    dependency_provider.add_dependencies(662, 17, vec![]);
    dependency_provider.add_dependencies(662, 16, vec![]);
    dependency_provider.add_dependencies(662, 15, vec![]);
    dependency_provider.add_dependencies(662, 8, vec![]);
    dependency_provider.add_dependencies(662, 7, vec![]);
    dependency_provider.add_dependencies(662, 6, vec![]);
    dependency_provider.add_dependencies(662, 3, vec![]);
    dependency_provider.add_dependencies(662, 2, vec![]);
    dependency_provider.add_dependencies(660, 15, vec![]);
    dependency_provider.add_dependencies(660, 9, vec![(662, Range::between(3, 19))]);
    dependency_provider.add_dependencies(660, 3, vec![]);
    dependency_provider.add_dependencies(660, 1, vec![]);
    dependency_provider.add_dependencies(650, 16, vec![(660, Range::any())]);
    dependency_provider.add_dependencies(650, 14, vec![]);
    dependency_provider.add_dependencies(650, 7, vec![]);
    dependency_provider.add_dependencies(650, 2, vec![]);
    dependency_provider.add_dependencies(650, 0, vec![]);
    dependency_provider.add_dependencies(647, 11, vec![]);
    dependency_provider.add_dependencies(647, 3, vec![]);
    dependency_provider.add_dependencies(645, 17, vec![]);
    dependency_provider.add_dependencies(645, 16, vec![]);
    dependency_provider.add_dependencies(645, 6, vec![]);
    dependency_provider.add_dependencies(645, 5, vec![]);
    dependency_provider.add_dependencies(645, 2, vec![(660, Range::higher_than(3))]);
    dependency_provider.add_dependencies(645, 0, vec![]);
    dependency_provider.add_dependencies(635, 18, vec![]);
    dependency_provider.add_dependencies(635, 17, vec![]);
    dependency_provider.add_dependencies(635, 15, vec![]);
    dependency_provider.add_dependencies(635, 14, vec![(662, Range::between(15, 19))]);
    dependency_provider.add_dependencies(635, 9, vec![]);
    dependency_provider.add_dependencies(635, 5, vec![]);
    dependency_provider.add_dependencies(635, 4, vec![(660, Range::any())]);
    dependency_provider.add_dependencies(635, 1, vec![]);
    dependency_provider.add_dependencies(635, 0, vec![]);
    dependency_provider.add_dependencies(627, 19, vec![(635, Range::higher_than(4))]);
    dependency_provider.add_dependencies(627, 18, vec![]);
    dependency_provider.add_dependencies(627, 17, vec![(662, Range::between(18, 19))]);
    dependency_provider.add_dependencies(627, 15, vec![(650, Range::between(2, 15))]);
    dependency_provider.add_dependencies(627, 14, vec![]);
    dependency_provider.add_dependencies(627, 12, vec![]);
    dependency_provider.add_dependencies(627, 11, vec![]);
    dependency_provider.add_dependencies(627, 7, vec![]);
    dependency_provider.add_dependencies(627, 3, vec![]);
    dependency_provider.add_dependencies(627, 1, vec![]);
    dependency_provider.add_dependencies(625, 10, vec![]);
    dependency_provider.add_dependencies(625, 8, vec![]);
    dependency_provider.add_dependencies(625, 6, vec![(647, Range::any())]);
    dependency_provider.add_dependencies(625, 4, vec![]);
    dependency_provider.add_dependencies(625, 1, vec![(627, Range::between(12, 19))]);
    dependency_provider.add_dependencies(619, 15, vec![]);
    dependency_provider.add_dependencies(
        619,
        14,
        vec![(662, Range::between(0, 8)), (645, Range::higher_than(2))],
    );
    dependency_provider.add_dependencies(619, 10, vec![]);
    dependency_provider.add_dependencies(619, 8, vec![]);
    dependency_provider.add_dependencies(619, 6, vec![]);
    dependency_provider.add_dependencies(619, 4, vec![(650, Range::any())]);
    dependency_provider.add_dependencies(619, 2, vec![(625, Range::higher_than(6))]);
    dependency_provider.add_dependencies(619, 1, vec![]);
    dependency_provider.add_dependencies(
        613,
        16,
        vec![(619, Range::between(4, 9)), (635, Range::between(14, 18))],
    );
    dependency_provider.add_dependencies(613, 10, vec![(635, Range::between(1, 16))]);
    dependency_provider.add_dependencies(
        613,
        8,
        vec![(627, Range::between(7, 15)), (662, Range::between(0, 4))],
    );
    dependency_provider.add_dependencies(613, 3, vec![]);
    dependency_provider.add_dependencies(608, 17, vec![]);
    dependency_provider.add_dependencies(608, 12, vec![(662, Range::higher_than(17))]);
    dependency_provider.add_dependencies(608, 9, vec![]);
    dependency_provider.add_dependencies(608, 6, vec![(635, Range::between(5, 18))]);
    dependency_provider.add_dependencies(608, 4, vec![]);
    dependency_provider.add_dependencies(606, 19, vec![(662, Range::between(6, 16))]);
    dependency_provider.add_dependencies(606, 16, vec![]);
    dependency_provider.add_dependencies(606, 7, vec![]);
    dependency_provider.add_dependencies(606, 6, vec![]);
    dependency_provider.add_dependencies(606, 3, vec![(619, Range::between(8, 15))]);
    dependency_provider.add_dependencies(601, 19, vec![]);
    dependency_provider.add_dependencies(601, 18, vec![]);
    dependency_provider.add_dependencies(601, 17, vec![]);
    dependency_provider.add_dependencies(601, 16, vec![(650, Range::between(0, 15))]);
    dependency_provider.add_dependencies(601, 15, vec![(608, Range::between(9, 13))]);
    dependency_provider.add_dependencies(601, 14, vec![]);
    dependency_provider.add_dependencies(601, 12, vec![]);
    dependency_provider.add_dependencies(601, 8, vec![]);
    dependency_provider.add_dependencies(601, 6, vec![(650, Range::higher_than(2))]);
    dependency_provider.add_dependencies(601, 5, vec![(650, Range::higher_than(14))]);
    dependency_provider.add_dependencies(601, 4, vec![]);
    dependency_provider.add_dependencies(601, 1, vec![(625, Range::between(6, 7))]);
    dependency_provider.add_dependencies(601, 0, vec![(608, Range::between(0, 10))]);
    dependency_provider.add_dependencies(
        600,
        18,
        vec![(635, Range::between(1, 6)), (601, Range::between(5, 9))],
    );
    dependency_provider.add_dependencies(600, 12, vec![]);
    dependency_provider.add_dependencies(600, 9, vec![(660, Range::between(3, 4))]);
    dependency_provider.add_dependencies(600, 8, vec![]);
    dependency_provider.add_dependencies(600, 7, vec![]);
    dependency_provider.add_dependencies(600, 6, vec![]);
    dependency_provider.add_dependencies(600, 5, vec![(625, Range::higher_than(8))]);
    dependency_provider.add_dependencies(599, 18, vec![(627, Range::between(15, 18))]);
    dependency_provider.add_dependencies(599, 14, vec![(601, Range::between(15, 18))]);
    dependency_provider.add_dependencies(599, 13, vec![(600, Range::higher_than(6))]);
    dependency_provider.add_dependencies(599, 12, vec![]);
    dependency_provider.add_dependencies(599, 10, vec![(662, Range::between(3, 7))]);
    dependency_provider.add_dependencies(599, 7, vec![]);
    dependency_provider.add_dependencies(599, 6, vec![(625, Range::between(6, 9))]);
    dependency_provider.add_dependencies(599, 2, vec![]);
    dependency_provider.add_dependencies(599, 1, vec![(635, Range::between(0, 10))]);
    dependency_provider.add_dependencies(599, 0, vec![(635, Range::between(1, 10))]);
    dependency_provider.add_dependencies(595, 19, vec![]);
    dependency_provider.add_dependencies(595, 16, vec![]);
    dependency_provider.add_dependencies(595, 15, vec![]);
    dependency_provider.add_dependencies(595, 14, vec![(619, Range::between(2, 15))]);
    dependency_provider.add_dependencies(595, 13, vec![(599, Range::between(7, 11))]);
    dependency_provider.add_dependencies(595, 10, vec![]);
    dependency_provider.add_dependencies(595, 2, vec![]);
    dependency_provider.add_dependencies(595, 1, vec![]);
    dependency_provider.add_dependencies(594, 12, vec![]);
    dependency_provider.add_dependencies(
        594,
        11,
        vec![(619, Range::higher_than(2)), (662, Range::between(6, 16))],
    );
    dependency_provider.add_dependencies(594, 5, vec![]);
    dependency_provider.add_dependencies(594, 4, vec![]);
    dependency_provider.add_dependencies(
        594,
        1,
        vec![(647, Range::between(0, 4)), (625, Range::higher_than(10))],
    );
    dependency_provider.add_dependencies(
        594,
        0,
        vec![
            (662, Range::between(16, 19)),
            (613, Range::higher_than(8)),
            (619, Range::higher_than(2)),
        ],
    );
    dependency_provider.add_dependencies(593, 15, vec![(635, Range::between(9, 15))]);
    dependency_provider.add_dependencies(593, 13, vec![]);
    dependency_provider.add_dependencies(593, 12, vec![]);
    dependency_provider.add_dependencies(593, 11, vec![]);
    dependency_provider.add_dependencies(593, 9, vec![]);
    dependency_provider.add_dependencies(593, 8, vec![]);
    dependency_provider.add_dependencies(593, 6, vec![(625, Range::any())]);
    dependency_provider.add_dependencies(593, 4, vec![(650, Range::higher_than(14))]);
    dependency_provider.add_dependencies(
        593,
        0,
        vec![(595, Range::between(0, 14)), (608, Range::higher_than(12))],
    );
    dependency_provider.add_dependencies(
        589,
        15,
        vec![(599, Range::higher_than(14)), (645, Range::between(2, 6))],
    );
    dependency_provider.add_dependencies(589, 10, vec![]);
    dependency_provider.add_dependencies(589, 8, vec![(600, Range::higher_than(18))]);
    dependency_provider.add_dependencies(589, 7, vec![(635, Range::between(0, 2))]);
    dependency_provider.add_dependencies(
        589,
        6,
        vec![(660, Range::between(3, 10)), (619, Range::between(8, 9))],
    );
    dependency_provider.add_dependencies(589, 1, vec![]);
    dependency_provider.add_dependencies(589, 0, vec![(595, Range::higher_than(14))]);
    dependency_provider.add_dependencies(584, 15, vec![]);
    dependency_provider.add_dependencies(584, 11, vec![]);
    dependency_provider.add_dependencies(584, 3, vec![]);
    dependency_provider.add_dependencies(
        584,
        1,
        vec![(600, Range::between(7, 13)), (599, Range::between(0, 2))],
    );
    dependency_provider.add_dependencies(576, 19, vec![]);
    dependency_provider.add_dependencies(576, 17, vec![(625, Range::between(4, 7))]);
    dependency_provider.add_dependencies(576, 16, vec![(599, Range::higher_than(14))]);
    dependency_provider.add_dependencies(
        576,
        15,
        vec![(594, Range::between(0, 12)), (593, Range::between(4, 12))],
    );
    dependency_provider.add_dependencies(576, 10, vec![(601, Range::between(0, 15))]);
    dependency_provider.add_dependencies(576, 8, vec![]);
    dependency_provider.add_dependencies(576, 7, vec![]);
    dependency_provider.add_dependencies(576, 4, vec![(645, Range::higher_than(5))]);
    dependency_provider.add_dependencies(574, 19, vec![]);
    dependency_provider.add_dependencies(574, 18, vec![(650, Range::between(0, 8))]);
    dependency_provider.add_dependencies(
        574,
        14,
        vec![
            (601, Range::higher_than(4)),
            (576, Range::between(15, 17)),
            (593, Range::higher_than(8)),
        ],
    );
    dependency_provider.add_dependencies(
        574,
        11,
        vec![(625, Range::between(4, 5)), (601, Range::between(8, 16))],
    );
    dependency_provider.add_dependencies(574, 10, vec![]);
    dependency_provider.add_dependencies(574, 9, vec![]);
    dependency_provider.add_dependencies(574, 8, vec![(635, Range::between(4, 18))]);
    dependency_provider.add_dependencies(574, 4, vec![]);
    dependency_provider.add_dependencies(
        574,
        2,
        vec![(650, Range::between(14, 15)), (589, Range::higher_than(6))],
    );
    dependency_provider.add_dependencies(574, 1, vec![(600, Range::higher_than(9))]);
    dependency_provider.add_dependencies(574, 0, vec![(593, Range::between(0, 5))]);
    dependency_provider.add_dependencies(
        569,
        19,
        vec![(574, Range::higher_than(10)), (589, Range::between(8, 11))],
    );
    dependency_provider.add_dependencies(
        569,
        18,
        vec![
            (647, Range::any()),
            (645, Range::between(16, 17)),
            (574, Range::between(14, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        569,
        16,
        vec![(645, Range::between(0, 17)), (599, Range::between(2, 8))],
    );
    dependency_provider.add_dependencies(569, 13, vec![(589, Range::higher_than(10))]);
    dependency_provider.add_dependencies(569, 12, vec![(627, Range::between(0, 13))]);
    dependency_provider.add_dependencies(569, 9, vec![(584, Range::higher_than(11))]);
    dependency_provider.add_dependencies(
        569,
        8,
        vec![(647, Range::any()), (635, Range::between(4, 16))],
    );
    dependency_provider.add_dependencies(
        569,
        6,
        vec![
            (595, Range::between(0, 17)),
            (635, Range::between(9, 16)),
            (601, Range::between(8, 13)),
        ],
    );
    dependency_provider.add_dependencies(569, 3, vec![(574, Range::between(0, 10))]);
    dependency_provider.add_dependencies(
        562,
        18,
        vec![(662, Range::between(8, 9)), (635, Range::between(5, 15))],
    );
    dependency_provider.add_dependencies(562, 17, vec![]);
    dependency_provider.add_dependencies(
        562,
        14,
        vec![(569, Range::between(0, 10)), (635, Range::any())],
    );
    dependency_provider.add_dependencies(562, 12, vec![]);
    dependency_provider.add_dependencies(
        562,
        11,
        vec![(574, Range::higher_than(8)), (647, Range::any())],
    );
    dependency_provider.add_dependencies(562, 10, vec![]);
    dependency_provider.add_dependencies(
        562,
        8,
        vec![(645, Range::between(0, 3)), (606, Range::between(0, 17))],
    );
    dependency_provider.add_dependencies(562, 4, vec![(608, Range::between(0, 5))]);
    dependency_provider.add_dependencies(
        562,
        0,
        vec![(593, Range::higher_than(6)), (576, Range::between(16, 17))],
    );
    dependency_provider.add_dependencies(560, 16, vec![]);
    dependency_provider.add_dependencies(
        560,
        8,
        vec![(635, Range::higher_than(4)), (600, Range::between(8, 13))],
    );
    dependency_provider.add_dependencies(560, 6, vec![(600, Range::between(0, 8))]);
    dependency_provider.add_dependencies(560, 4, vec![]);
    dependency_provider.add_dependencies(560, 2, vec![(593, Range::between(0, 13))]);
    dependency_provider.add_dependencies(
        560,
        0,
        vec![(613, Range::between(0, 9)), (589, Range::between(1, 11))],
    );
    dependency_provider.add_dependencies(559, 18, vec![(601, Range::between(1, 15))]);
    dependency_provider.add_dependencies(559, 12, vec![]);
    dependency_provider.add_dependencies(559, 11, vec![(574, Range::higher_than(11))]);
    dependency_provider.add_dependencies(559, 10, vec![]);
    dependency_provider.add_dependencies(559, 9, vec![(560, Range::between(4, 9))]);
    dependency_provider.add_dependencies(559, 8, vec![(650, Range::higher_than(14))]);
    dependency_provider.add_dependencies(547, 19, vec![]);
    dependency_provider.add_dependencies(
        547,
        18,
        vec![(569, Range::any()), (662, Range::between(0, 8))],
    );
    dependency_provider.add_dependencies(
        547,
        17,
        vec![
            (662, Range::between(3, 19)),
            (650, Range::between(7, 15)),
            (574, Range::between(9, 15)),
        ],
    );
    dependency_provider.add_dependencies(547, 16, vec![]);
    dependency_provider.add_dependencies(547, 14, vec![(601, Range::between(15, 17))]);
    dependency_provider.add_dependencies(
        547,
        13,
        vec![(601, Range::between(4, 16)), (589, Range::between(8, 11))],
    );
    dependency_provider.add_dependencies(547, 12, vec![]);
    dependency_provider.add_dependencies(
        547,
        11,
        vec![
            (662, Range::between(0, 19)),
            (574, Range::between(1, 5)),
            (584, Range::between(3, 4)),
        ],
    );
    dependency_provider.add_dependencies(547, 6, vec![]);
    dependency_provider.add_dependencies(547, 5, vec![]);
    dependency_provider.add_dependencies(
        547,
        4,
        vec![(635, Range::between(5, 15)), (645, Range::between(16, 17))],
    );
    dependency_provider.add_dependencies(547, 3, vec![]);
    dependency_provider.add_dependencies(547, 2, vec![(635, Range::between(0, 15))]);
    dependency_provider.add_dependencies(547, 0, vec![(650, Range::between(2, 15))]);
    dependency_provider.add_dependencies(541, 17, vec![(650, Range::higher_than(14))]);
    dependency_provider.add_dependencies(541, 16, vec![(645, Range::between(2, 7))]);
    dependency_provider.add_dependencies(
        541,
        15,
        vec![
            (576, Range::higher_than(10)),
            (662, Range::between(0, 4)),
            (547, Range::between(0, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        541,
        14,
        vec![
            (547, Range::higher_than(6)),
            (599, Range::between(2, 13)),
            (627, Range::between(17, 19)),
        ],
    );
    dependency_provider.add_dependencies(
        541,
        13,
        vec![
            (589, Range::higher_than(1)),
            (574, Range::between(2, 15)),
            (635, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(541, 8, vec![(625, Range::between(4, 7))]);
    dependency_provider.add_dependencies(
        541,
        7,
        vec![
            (593, Range::between(4, 9)),
            (547, Range::between(4, 19)),
            (650, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(541, 6, vec![]);
    dependency_provider.add_dependencies(541, 1, vec![(601, Range::between(1, 15))]);
    dependency_provider.add_dependencies(
        535,
        16,
        vec![(627, Range::higher_than(18)), (625, Range::between(4, 9))],
    );
    dependency_provider.add_dependencies(535, 15, vec![(608, Range::between(0, 7))]);
    dependency_provider.add_dependencies(
        535,
        13,
        vec![(662, Range::between(7, 9)), (635, Range::between(5, 10))],
    );
    dependency_provider.add_dependencies(535, 12, vec![]);
    dependency_provider.add_dependencies(535, 8, vec![]);
    dependency_provider.add_dependencies(535, 6, vec![(541, Range::any())]);
    dependency_provider.add_dependencies(
        535,
        5,
        vec![(547, Range::between(2, 14)), (559, Range::between(0, 10))],
    );
    dependency_provider.add_dependencies(535, 1, vec![(599, Range::between(6, 11))]);
    dependency_provider.add_dependencies(
        523,
        19,
        vec![(535, Range::between(0, 14)), (650, Range::any())],
    );
    dependency_provider.add_dependencies(
        523,
        18,
        vec![
            (619, Range::between(0, 5)),
            (627, Range::between(14, 16)),
            (625, Range::higher_than(4)),
        ],
    );
    dependency_provider.add_dependencies(523, 14, vec![(645, Range::higher_than(16))]);
    dependency_provider.add_dependencies(523, 13, vec![(593, Range::between(6, 14))]);
    dependency_provider.add_dependencies(
        523,
        11,
        vec![
            (619, Range::between(4, 5)),
            (576, Range::between(8, 16)),
            (535, Range::higher_than(8)),
        ],
    );
    dependency_provider.add_dependencies(523, 10, vec![]);
    dependency_provider.add_dependencies(
        523,
        9,
        vec![(562, Range::higher_than(17)), (569, Range::between(6, 17))],
    );
    dependency_provider.add_dependencies(
        523,
        8,
        vec![(600, Range::between(6, 10)), (594, Range::between(0, 12))],
    );
    dependency_provider.add_dependencies(523, 6, vec![]);
    dependency_provider.add_dependencies(
        523,
        1,
        vec![(608, Range::between(0, 13)), (662, Range::between(17, 19))],
    );
    dependency_provider.add_dependencies(523, 0, vec![(613, Range::any())]);
    dependency_provider.add_dependencies(505, 18, vec![(547, Range::between(6, 12))]);
    dependency_provider.add_dependencies(
        505,
        17,
        vec![(619, Range::between(6, 9)), (589, Range::between(7, 8))],
    );
    dependency_provider.add_dependencies(
        505,
        13,
        vec![
            (595, Range::between(0, 16)),
            (600, Range::between(0, 13)),
            (589, Range::between(0, 8)),
            (619, Range::between(14, 15)),
            (584, Range::between(11, 12)),
        ],
    );
    dependency_provider.add_dependencies(
        505,
        10,
        vec![(595, Range::higher_than(10)), (589, Range::between(1, 2))],
    );
    dependency_provider.add_dependencies(505, 9, vec![]);
    dependency_provider.add_dependencies(505, 6, vec![(559, Range::between(11, 12))]);
    dependency_provider.add_dependencies(
        505,
        2,
        vec![
            (662, Range::between(7, 19)),
            (619, Range::between(0, 5)),
            (535, Range::between(8, 13)),
        ],
    );
    dependency_provider.add_dependencies(505, 1, vec![(666, Range::any())]);
    dependency_provider.add_dependencies(505, 0, vec![(535, Range::between(0, 14))]);
    dependency_provider.add_dependencies(
        500,
        16,
        vec![(627, Range::higher_than(17)), (601, Range::between(6, 13))],
    );
    dependency_provider.add_dependencies(500, 12, vec![]);
    dependency_provider.add_dependencies(500, 5, vec![(594, Range::any())]);
    dependency_provider.add_dependencies(500, 2, vec![(613, Range::higher_than(8))]);
    dependency_provider.add_dependencies(495, 16, vec![(608, Range::higher_than(9))]);
    dependency_provider.add_dependencies(495, 12, vec![(662, Range::between(0, 4))]);
    dependency_provider.add_dependencies(
        495,
        11,
        vec![
            (594, Range::higher_than(1)),
            (547, Range::between(6, 13)),
            (613, Range::higher_than(8)),
            (645, Range::between(0, 7)),
            (559, Range::between(8, 12)),
        ],
    );
    dependency_provider.add_dependencies(495, 9, vec![]);
    dependency_provider.add_dependencies(495, 8, vec![(576, Range::between(0, 9))]);
    dependency_provider.add_dependencies(
        495,
        5,
        vec![(595, Range::between(15, 17)), (650, Range::between(14, 15))],
    );
    dependency_provider.add_dependencies(495, 4, vec![]);
    dependency_provider.add_dependencies(495, 3, vec![(608, Range::between(6, 13))]);
    dependency_provider.add_dependencies(495, 1, vec![]);
    dependency_provider.add_dependencies(
        494,
        12,
        vec![
            (645, Range::between(6, 7)),
            (601, Range::higher_than(16)),
            (495, Range::higher_than(11)),
        ],
    );
    dependency_provider.add_dependencies(
        494,
        5,
        vec![(574, Range::higher_than(8)), (601, Range::between(0, 19))],
    );
    dependency_provider.add_dependencies(
        494,
        1,
        vec![
            (547, Range::between(11, 19)),
            (619, Range::between(6, 11)),
            (599, Range::between(2, 8)),
            (562, Range::higher_than(10)),
        ],
    );
    dependency_provider.add_dependencies(
        491,
        18,
        vec![
            (595, Range::between(10, 17)),
            (569, Range::between(6, 13)),
            (594, Range::between(5, 6)),
            (662, Range::higher_than(17)),
            (635, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(491, 9, vec![(662, Range::higher_than(6))]);
    dependency_provider.add_dependencies(491, 6, vec![(662, Range::between(16, 18))]);
    dependency_provider.add_dependencies(
        491,
        5,
        vec![(647, Range::between(0, 4)), (595, Range::between(14, 16))],
    );
    dependency_provider.add_dependencies(491, 3, vec![(547, Range::between(17, 19))]);
    dependency_provider.add_dependencies(
        491,
        2,
        vec![(595, Range::higher_than(16)), (662, Range::between(15, 17))],
    );
    dependency_provider.add_dependencies(491, 0, vec![]);
    dependency_provider.add_dependencies(
        484,
        18,
        vec![
            (574, Range::between(0, 12)),
            (495, Range::between(5, 10)),
            (601, Range::between(0, 19)),
        ],
    );
    dependency_provider.add_dependencies(484, 17, vec![(635, Range::between(9, 18))]);
    dependency_provider.add_dependencies(
        484,
        16,
        vec![
            (600, Range::higher_than(18)),
            (613, Range::between(0, 4)),
            (491, Range::higher_than(3)),
        ],
    );
    dependency_provider.add_dependencies(484, 14, vec![(645, Range::any())]);
    dependency_provider.add_dependencies(484, 11, vec![(541, Range::between(6, 17))]);
    dependency_provider.add_dependencies(484, 10, vec![]);
    dependency_provider.add_dependencies(484, 8, vec![(541, Range::higher_than(16))]);
    dependency_provider.add_dependencies(
        484,
        7,
        vec![(613, Range::between(0, 4)), (491, Range::between(3, 4))],
    );
    dependency_provider.add_dependencies(484, 6, vec![(662, Range::between(15, 17))]);
    dependency_provider.add_dependencies(484, 4, vec![(574, Range::between(0, 1))]);
    dependency_provider.add_dependencies(
        484,
        2,
        vec![
            (505, Range::between(9, 14)),
            (574, Range::between(2, 5)),
            (560, Range::between(2, 9)),
            (535, Range::higher_than(13)),
        ],
    );
    dependency_provider.add_dependencies(484, 0, vec![(594, Range::between(0, 12))]);
    dependency_provider.add_dependencies(
        479,
        14,
        vec![(484, Range::between(0, 8)), (589, Range::between(1, 7))],
    );
    dependency_provider.add_dependencies(479, 5, vec![(523, Range::higher_than(10))]);
    dependency_provider.add_dependencies(
        477,
        19,
        vec![
            (584, Range::between(0, 12)),
            (600, Range::between(9, 13)),
            (608, Range::between(0, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        477,
        18,
        vec![
            (495, Range::between(3, 10)),
            (574, Range::between(1, 19)),
            (562, Range::between(4, 15)),
            (613, Range::between(8, 11)),
        ],
    );
    dependency_provider.add_dependencies(477, 15, vec![(635, Range::between(5, 18))]);
    dependency_provider.add_dependencies(
        477,
        13,
        vec![
            (595, Range::between(13, 16)),
            (535, Range::between(5, 7)),
            (495, Range::between(3, 6)),
        ],
    );
    dependency_provider.add_dependencies(477, 12, vec![]);
    dependency_provider.add_dependencies(
        477,
        9,
        vec![
            (569, Range::between(8, 19)),
            (660, Range::between(0, 4)),
            (608, Range::between(9, 13)),
            (484, Range::between(2, 15)),
            (645, Range::between(5, 7)),
            (650, Range::between(7, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        477,
        7,
        vec![(569, Range::between(0, 19)), (600, Range::any())],
    );
    dependency_provider.add_dependencies(
        477,
        4,
        vec![(541, Range::between(14, 16)), (601, Range::between(0, 1))],
    );
    dependency_provider.add_dependencies(
        477,
        2,
        vec![(484, Range::between(10, 12)), (608, Range::between(0, 13))],
    );
    dependency_provider.add_dependencies(477, 1, vec![(523, Range::between(0, 15))]);
    dependency_provider.add_dependencies(
        477,
        0,
        vec![(505, Range::between(1, 18)), (562, Range::between(0, 13))],
    );
    dependency_provider.add_dependencies(475, 9, vec![(593, Range::between(4, 13))]);
    dependency_provider.add_dependencies(
        475,
        3,
        vec![(599, Range::between(1, 8)), (619, Range::between(2, 15))],
    );
    dependency_provider.add_dependencies(
        475,
        0,
        vec![
            (569, Range::higher_than(12)),
            (608, Range::between(6, 10)),
            (500, Range::higher_than(16)),
            (560, Range::higher_than(4)),
            (589, Range::between(8, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        471,
        18,
        vec![
            (662, Range::between(8, 18)),
            (576, Range::between(7, 18)),
            (523, Range::between(9, 10)),
            (484, Range::between(4, 18)),
        ],
    );
    dependency_provider.add_dependencies(471, 2, vec![(608, Range::any()), (494, Range::any())]);
    dependency_provider.add_dependencies(
        471,
        1,
        vec![
            (599, Range::between(1, 13)),
            (484, Range::between(2, 9)),
            (569, Range::between(8, 10)),
        ],
    );
    dependency_provider.add_dependencies(462, 16, vec![(608, Range::between(0, 7))]);
    dependency_provider.add_dependencies(462, 14, vec![(491, Range::between(3, 7))]);
    dependency_provider.add_dependencies(
        462,
        12,
        vec![
            (593, Range::between(8, 14)),
            (574, Range::higher_than(19)),
            (477, Range::between(1, 8)),
            (547, Range::between(4, 18)),
        ],
    );
    dependency_provider.add_dependencies(
        462,
        7,
        vec![
            (606, Range::between(16, 17)),
            (484, Range::between(2, 18)),
            (574, Range::between(4, 19)),
        ],
    );
    dependency_provider.add_dependencies(
        462,
        6,
        vec![
            (535, Range::higher_than(6)),
            (613, Range::between(8, 11)),
            (599, Range::between(12, 14)),
            (608, Range::between(0, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        462,
        4,
        vec![(635, Range::between(1, 5)), (479, Range::between(0, 6))],
    );
    dependency_provider.add_dependencies(
        462,
        3,
        vec![(608, Range::higher_than(9)), (589, Range::between(8, 11))],
    );
    dependency_provider.add_dependencies(
        462,
        2,
        vec![
            (601, Range::between(4, 17)),
            (606, Range::higher_than(16)),
            (625, Range::between(4, 9)),
            (574, Range::between(2, 12)),
            (477, Range::between(0, 10)),
        ],
    );
    dependency_provider.add_dependencies(462, 1, vec![]);
    dependency_provider.add_dependencies(455, 19, vec![]);
    dependency_provider.add_dependencies(
        455,
        17,
        vec![
            (569, Range::between(13, 19)),
            (491, Range::between(9, 10)),
            (662, Range::between(17, 19)),
        ],
    );
    dependency_provider.add_dependencies(
        455,
        16,
        vec![
            (650, Range::higher_than(2)),
            (599, Range::between(2, 3)),
            (495, Range::between(3, 4)),
        ],
    );
    dependency_provider.add_dependencies(455, 15, vec![(666, Range::any())]);
    dependency_provider.add_dependencies(455, 12, vec![(666, Range::any())]);
    dependency_provider.add_dependencies(
        455,
        10,
        vec![
            (613, Range::higher_than(16)),
            (662, Range::between(3, 9)),
            (600, Range::between(6, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        455,
        9,
        vec![
            (595, Range::between(0, 14)),
            (601, Range::between(0, 6)),
            (562, Range::between(14, 18)),
        ],
    );
    dependency_provider.add_dependencies(
        455,
        8,
        vec![
            (599, Range::between(7, 14)),
            (560, Range::higher_than(16)),
            (574, Range::between(4, 12)),
        ],
    );
    dependency_provider.add_dependencies(
        455,
        7,
        vec![
            (574, Range::between(2, 15)),
            (595, Range::between(0, 2)),
            (484, Range::between(14, 17)),
        ],
    );
    dependency_provider.add_dependencies(455, 4, vec![(600, Range::between(0, 13))]);
    dependency_provider.add_dependencies(
        455,
        2,
        vec![
            (560, Range::any()),
            (491, Range::between(0, 1)),
            (559, Range::between(10, 12)),
        ],
    );
    dependency_provider.add_dependencies(
        450,
        19,
        vec![
            (471, Range::higher_than(2)),
            (662, Range::between(7, 9)),
            (645, Range::higher_than(2)),
            (523, Range::between(10, 14)),
            (576, Range::between(10, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        450,
        17,
        vec![
            (599, Range::between(0, 2)),
            (560, Range::between(0, 9)),
            (562, Range::higher_than(11)),
        ],
    );
    dependency_provider.add_dependencies(
        450,
        16,
        vec![
            (535, Range::between(12, 14)),
            (594, Range::between(4, 12)),
            (500, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        450,
        14,
        vec![
            (562, Range::between(12, 15)),
            (619, Range::between(2, 7)),
            (495, Range::between(9, 12)),
            (471, Range::higher_than(2)),
        ],
    );
    dependency_provider.add_dependencies(450, 13, vec![(495, Range::between(3, 5))]);
    dependency_provider.add_dependencies(450, 10, vec![]);
    dependency_provider.add_dependencies(
        450,
        7,
        vec![
            (593, Range::higher_than(9)),
            (645, Range::higher_than(16)),
            (477, Range::between(0, 5)),
        ],
    );
    dependency_provider.add_dependencies(450, 6, vec![(645, Range::higher_than(5))]);
    dependency_provider.add_dependencies(
        450,
        5,
        vec![
            (594, Range::higher_than(4)),
            (484, Range::between(8, 17)),
            (576, Range::higher_than(8)),
            (547, Range::between(4, 13)),
        ],
    );
    dependency_provider.add_dependencies(450, 4, vec![]);
    dependency_provider.add_dependencies(450, 3, vec![(505, Range::between(1, 2))]);
    dependency_provider.add_dependencies(450, 2, vec![(601, Range::between(1, 18))]);
    dependency_provider.add_dependencies(
        448,
        19,
        vec![(601, Range::between(4, 7)), (484, Range::between(7, 8))],
    );
    dependency_provider.add_dependencies(448, 16, vec![(660, Range::between(0, 4))]);
    dependency_provider.add_dependencies(448, 15, vec![]);
    dependency_provider.add_dependencies(
        448,
        13,
        vec![(650, Range::between(14, 15)), (455, Range::higher_than(17))],
    );
    dependency_provider.add_dependencies(448, 12, vec![(562, Range::between(0, 15))]);
    dependency_provider.add_dependencies(
        448,
        11,
        vec![
            (660, Range::any()),
            (505, Range::between(1, 7)),
            (477, Range::any()),
            (450, Range::between(0, 17)),
            (455, Range::between(10, 16)),
            (625, Range::between(4, 7)),
            (601, Range::between(1, 5)),
            (650, Range::between(0, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        448,
        3,
        vec![(662, Range::between(6, 18)), (562, Range::between(8, 11))],
    );
    dependency_provider.add_dependencies(
        447,
        14,
        vec![
            (541, Range::between(6, 15)),
            (500, Range::any()),
            (450, Range::between(6, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        447,
        13,
        vec![(606, Range::between(6, 17)), (662, Range::higher_than(6))],
    );
    dependency_provider.add_dependencies(447, 11, vec![(541, Range::between(6, 9))]);
    dependency_provider.add_dependencies(447, 8, vec![(645, Range::between(0, 1))]);
    dependency_provider.add_dependencies(447, 1, vec![(541, Range::higher_than(8))]);
    dependency_provider.add_dependencies(443, 17, vec![(569, Range::higher_than(16))]);
    dependency_provider.add_dependencies(
        443,
        14,
        vec![
            (595, Range::between(2, 14)),
            (450, Range::between(3, 15)),
            (574, Range::between(0, 2)),
        ],
    );
    dependency_provider.add_dependencies(
        443,
        12,
        vec![
            (606, Range::between(0, 7)),
            (523, Range::between(6, 19)),
            (562, Range::between(10, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        443,
        0,
        vec![
            (500, Range::any()),
            (505, Range::between(0, 18)),
            (447, Range::between(0, 12)),
            (547, Range::higher_than(3)),
        ],
    );
    dependency_provider.add_dependencies(
        441,
        16,
        vec![(589, Range::between(8, 11)), (560, Range::higher_than(8))],
    );
    dependency_provider.add_dependencies(
        441,
        15,
        vec![(505, Range::between(6, 14)), (455, Range::between(4, 5))],
    );
    dependency_provider.add_dependencies(
        441,
        13,
        vec![(445, Range::higher_than(8)), (569, Range::between(0, 13))],
    );
    dependency_provider.add_dependencies(
        441,
        8,
        vec![(462, Range::between(3, 8)), (491, Range::between(3, 6))],
    );
    dependency_provider.add_dependencies(441, 2, vec![]);
    dependency_provider.add_dependencies(
        437,
        16,
        vec![
            (547, Range::between(3, 17)),
            (475, Range::higher_than(9)),
            (593, Range::between(8, 13)),
            (450, Range::between(14, 17)),
            (500, Range::between(0, 3)),
        ],
    );
    dependency_provider.add_dependencies(
        437,
        15,
        vec![
            (600, Range::between(0, 8)),
            (635, Range::higher_than(9)),
            (535, Range::higher_than(8)),
            (593, Range::between(9, 12)),
            (541, Range::between(13, 16)),
        ],
    );
    dependency_provider.add_dependencies(437, 11, vec![(450, Range::between(6, 11))]);
    dependency_provider.add_dependencies(418, 19, vec![(505, Range::between(0, 2))]);
    dependency_provider.add_dependencies(
        418,
        18,
        vec![
            (595, Range::between(13, 17)),
            (576, Range::between(8, 9)),
            (437, Range::between(0, 16)),
        ],
    );
    dependency_provider.add_dependencies(418, 16, vec![(662, Range::any())]);
    dependency_provider.add_dependencies(
        418,
        15,
        vec![(450, Range::higher_than(4)), (441, Range::between(0, 16))],
    );
    dependency_provider.add_dependencies(
        418,
        8,
        vec![
            (601, Range::between(12, 18)),
            (574, Range::between(0, 12)),
            (484, Range::between(7, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        418,
        3,
        vec![
            (627, Range::between(7, 13)),
            (535, Range::between(13, 16)),
            (450, Range::between(3, 6)),
        ],
    );
    dependency_provider.add_dependencies(418, 0, vec![(627, Range::between(3, 15))]);
    dependency_provider.add_dependencies(
        410,
        18,
        vec![(477, Range::between(1, 14)), (595, Range::between(0, 14))],
    );
    dependency_provider.add_dependencies(
        410,
        16,
        vec![
            (484, Range::between(0, 15)),
            (523, Range::between(10, 14)),
            (601, Range::between(4, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        410,
        13,
        vec![(541, Range::higher_than(8)), (491, Range::between(5, 6))],
    );
    dependency_provider.add_dependencies(410, 9, vec![(569, Range::between(12, 19))]);
    dependency_provider.add_dependencies(410, 7, vec![]);
    dependency_provider.add_dependencies(
        410,
        5,
        vec![(447, Range::higher_than(8)), (477, Range::between(1, 13))],
    );
    dependency_provider.add_dependencies(
        410,
        3,
        vec![
            (475, Range::between(0, 4)),
            (660, Range::between(3, 10)),
            (484, Range::between(10, 18)),
            (505, Range::between(1, 18)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        17,
        vec![
            (491, Range::between(0, 6)),
            (505, Range::between(2, 10)),
            (450, Range::between(7, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        14,
        vec![(650, Range::between(7, 15)), (600, Range::between(0, 9))],
    );
    dependency_provider.add_dependencies(
        405,
        13,
        vec![
            (523, Range::between(9, 15)),
            (574, Range::higher_than(10)),
            (450, Range::between(3, 17)),
            (562, Range::between(11, 15)),
            (627, Range::between(7, 19)),
        ],
    );
    dependency_provider.add_dependencies(405, 11, vec![]);
    dependency_provider.add_dependencies(
        405,
        10,
        vec![(547, Range::between(5, 19)), (559, Range::between(0, 12))],
    );
    dependency_provider.add_dependencies(405, 9, vec![(635, Range::between(4, 10))]);
    dependency_provider.add_dependencies(
        405,
        7,
        vec![(562, Range::between(10, 15)), (547, Range::between(2, 12))],
    );
    dependency_provider.add_dependencies(
        405,
        6,
        vec![
            (635, Range::between(14, 16)),
            (594, Range::higher_than(12)),
            (495, Range::between(0, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        5,
        vec![
            (477, Range::higher_than(12)),
            (447, Range::between(8, 12)),
            (600, Range::higher_than(6)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        4,
        vec![
            (562, Range::between(8, 13)),
            (559, Range::any()),
            (462, Range::between(2, 7)),
            (495, Range::any()),
            (450, Range::between(4, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        2,
        vec![
            (484, Range::between(4, 9)),
            (410, Range::any()),
            (601, Range::between(12, 17)),
            (477, Range::between(12, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        1,
        vec![
            (484, Range::between(16, 17)),
            (662, Range::between(16, 18)),
            (594, Range::between(1, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        405,
        0,
        vec![
            (599, Range::between(7, 15)),
            (625, Range::between(6, 7)),
            (455, Range::between(8, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        400,
        9,
        vec![
            (599, Range::between(0, 3)),
            (600, Range::between(6, 8)),
            (462, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(400, 4, vec![(666, Range::any())]);
    dependency_provider.add_dependencies(
        400,
        0,
        vec![(462, Range::between(4, 5)), (443, Range::between(0, 13))],
    );
    dependency_provider.add_dependencies(
        396,
        19,
        vec![(593, Range::between(0, 5)), (443, Range::between(12, 15))],
    );
    dependency_provider.add_dependencies(
        396,
        18,
        vec![(662, Range::between(16, 18)), (443, Range::between(0, 1))],
    );
    dependency_provider.add_dependencies(396, 17, vec![(441, Range::higher_than(15))]);
    dependency_provider.add_dependencies(
        396,
        15,
        vec![
            (495, Range::between(11, 13)),
            (523, Range::between(8, 14)),
            (650, Range::between(0, 15)),
            (547, Range::between(11, 19)),
            (627, Range::higher_than(17)),
            (418, Range::higher_than(18)),
        ],
    );
    dependency_provider.add_dependencies(
        396,
        14,
        vec![(650, Range::between(2, 15)), (535, Range::higher_than(12))],
    );
    dependency_provider.add_dependencies(
        396,
        11,
        vec![(441, Range::between(8, 16)), (589, Range::between(7, 11))],
    );
    dependency_provider.add_dependencies(
        396,
        10,
        vec![
            (484, Range::between(0, 15)),
            (547, Range::between(11, 13)),
            (619, Range::between(2, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        396,
        9,
        vec![
            (523, Range::between(9, 10)),
            (660, Range::any()),
            (448, Range::between(15, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        396,
        7,
        vec![(562, Range::higher_than(12)), (608, Range::any())],
    );
    dependency_provider.add_dependencies(396, 5, vec![(484, Range::between(14, 18))]);
    dependency_provider.add_dependencies(
        396,
        4,
        vec![
            (627, Range::higher_than(17)),
            (535, Range::between(8, 14)),
            (589, Range::between(1, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        396,
        3,
        vec![
            (569, Range::between(0, 17)),
            (562, Range::between(8, 18)),
            (576, Range::higher_than(15)),
        ],
    );
    dependency_provider.add_dependencies(
        396,
        1,
        vec![
            (574, Range::between(1, 19)),
            (477, Range::between(4, 10)),
            (606, Range::higher_than(16)),
        ],
    );
    dependency_provider.add_dependencies(
        371,
        19,
        vec![
            (523, Range::between(11, 15)),
            (495, Range::between(0, 10)),
            (505, Range::between(0, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        371,
        18,
        vec![(455, Range::between(7, 13)), (627, Range::between(0, 16))],
    );
    dependency_provider.add_dependencies(
        371,
        13,
        vec![(400, Range::between(0, 1)), (601, Range::between(4, 15))],
    );
    dependency_provider.add_dependencies(
        371,
        12,
        vec![
            (475, Range::between(0, 4)),
            (443, Range::between(0, 15)),
            (562, Range::between(10, 18)),
            (547, Range::between(0, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        371,
        11,
        vec![
            (576, Range::between(0, 18)),
            (574, Range::between(2, 12)),
            (589, Range::between(6, 8)),
            (547, Range::between(12, 13)),
            (405, Range::between(5, 7)),
            (410, Range::between(0, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        371,
        9,
        vec![
            (645, Range::between(0, 7)),
            (477, Range::between(9, 16)),
            (400, Range::between(0, 1)),
            (455, Range::between(0, 9)),
        ],
    );
    dependency_provider.add_dependencies(364, 14, vec![(589, Range::between(7, 8))]);
    dependency_provider.add_dependencies(
        364,
        12,
        vec![
            (484, Range::between(8, 11)),
            (455, Range::between(0, 18)),
            (601, Range::between(1, 13)),
            (437, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(364, 11, vec![]);
    dependency_provider.add_dependencies(364, 10, vec![]);
    dependency_provider.add_dependencies(364, 9, vec![(595, Range::between(2, 15))]);
    dependency_provider.add_dependencies(364, 7, vec![(396, Range::between(3, 11))]);
    dependency_provider.add_dependencies(
        364,
        5,
        vec![(645, Range::higher_than(6)), (601, Range::between(4, 9))],
    );
    dependency_provider.add_dependencies(364, 4, vec![(523, Range::between(1, 7))]);
    dependency_provider.add_dependencies(364, 0, vec![(562, Range::between(8, 18))]);
    dependency_provider.add_dependencies(
        352,
        17,
        vec![
            (484, Range::between(16, 18)),
            (613, Range::higher_than(10)),
            (477, Range::between(12, 14)),
            (547, Range::between(0, 5)),
            (593, Range::between(4, 13)),
        ],
    );
    dependency_provider.add_dependencies(352, 16, vec![(613, Range::higher_than(8))]);
    dependency_provider.add_dependencies(
        352,
        15,
        vec![
            (477, Range::between(1, 14)),
            (594, Range::higher_than(1)),
            (560, Range::between(0, 7)),
        ],
    );
    dependency_provider.add_dependencies(
        352,
        14,
        vec![
            (437, Range::between(15, 16)),
            (584, Range::between(3, 12)),
            (647, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(352, 10, vec![(574, Range::between(8, 10))]);
    dependency_provider.add_dependencies(352, 9, vec![(601, Range::between(1, 6))]);
    dependency_provider.add_dependencies(
        352,
        6,
        vec![(562, Range::higher_than(12)), (477, Range::between(9, 14))],
    );
    dependency_provider.add_dependencies(352, 5, vec![]);
    dependency_provider.add_dependencies(
        349,
        17,
        vec![(396, Range::between(15, 18)), (455, Range::between(4, 16))],
    );
    dependency_provider.add_dependencies(
        349,
        14,
        vec![
            (385, Range::between(0, 6)),
            (576, Range::between(8, 18)),
            (662, Range::between(3, 8)),
            (608, Range::between(0, 7)),
            (589, Range::between(0, 8)),
        ],
    );
    dependency_provider.add_dependencies(349, 12, vec![(477, Range::between(0, 3))]);
    dependency_provider.add_dependencies(
        349,
        11,
        vec![(495, Range::between(0, 9)), (523, Range::between(8, 11))],
    );
    dependency_provider.add_dependencies(349, 10, vec![(627, Range::between(17, 18))]);
    dependency_provider.add_dependencies(
        349,
        9,
        vec![(405, Range::between(1, 2)), (396, Range::between(0, 15))],
    );
    dependency_provider.add_dependencies(
        349,
        8,
        vec![
            (547, Range::between(3, 13)),
            (445, Range::between(0, 6)),
            (450, Range::between(4, 8)),
            (619, Range::between(0, 5)),
            (600, Range::higher_than(18)),
            (541, Range::between(8, 9)),
            (601, Range::between(0, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        349,
        7,
        vec![
            (541, Range::between(7, 9)),
            (418, Range::higher_than(19)),
            (601, Range::higher_than(17)),
        ],
    );
    dependency_provider.add_dependencies(
        349,
        6,
        vec![(500, Range::between(0, 6)), (625, Range::between(6, 9))],
    );
    dependency_provider.add_dependencies(
        349,
        5,
        vec![
            (484, Range::between(4, 5)),
            (541, Range::between(15, 16)),
            (627, Range::between(7, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        349,
        4,
        vec![
            (547, Range::between(0, 3)),
            (450, Range::between(3, 6)),
            (505, Range::between(1, 3)),
        ],
    );
    dependency_provider.add_dependencies(349, 3, vec![]);
    dependency_provider.add_dependencies(
        349,
        2,
        vec![(600, Range::higher_than(9)), (396, Range::between(4, 12))],
    );
    dependency_provider.add_dependencies(
        349,
        1,
        vec![(593, Range::between(0, 12)), (559, Range::between(9, 13))],
    );
    dependency_provider.add_dependencies(
        349,
        0,
        vec![
            (352, Range::between(10, 17)),
            (599, Range::between(1, 14)),
            (601, Range::between(14, 17)),
        ],
    );
    dependency_provider.add_dependencies(348, 18, vec![]);
    dependency_provider.add_dependencies(348, 17, vec![(495, Range::higher_than(9))]);
    dependency_provider.add_dependencies(
        348,
        16,
        vec![
            (625, Range::between(0, 2)),
            (523, Range::between(8, 11)),
            (595, Range::higher_than(10)),
            (405, Range::between(4, 15)),
            (606, Range::between(7, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        348,
        15,
        vec![(500, Range::higher_than(16)), (477, Range::between(1, 3))],
    );
    dependency_provider.add_dependencies(
        348,
        14,
        vec![(437, Range::between(0, 12)), (594, Range::between(1, 12))],
    );
    dependency_provider.add_dependencies(
        348,
        13,
        vec![
            (635, Range::higher_than(1)),
            (650, Range::higher_than(7)),
            (600, Range::between(6, 10)),
        ],
    );
    dependency_provider.add_dependencies(
        348,
        12,
        vec![(599, Range::between(1, 8)), (541, Range::higher_than(16))],
    );
    dependency_provider.add_dependencies(348, 11, vec![(494, Range::between(0, 2))]);
    dependency_provider.add_dependencies(
        348,
        10,
        vec![(349, Range::between(5, 13)), (471, Range::between(2, 3))],
    );
    dependency_provider.add_dependencies(
        348,
        9,
        vec![
            (574, Range::higher_than(1)),
            (418, Range::between(0, 19)),
            (505, Range::between(0, 11)),
            (477, Range::higher_than(18)),
        ],
    );
    dependency_provider.add_dependencies(
        348,
        8,
        vec![
            (562, Range::between(10, 12)),
            (594, Range::higher_than(1)),
            (547, Range::between(2, 4)),
            (505, Range::higher_than(13)),
            (560, Range::higher_than(2)),
            (559, Range::between(8, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        348,
        7,
        vec![
            (594, Range::between(4, 6)),
            (370, Range::higher_than(16)),
            (500, Range::between(5, 13)),
            (418, Range::higher_than(19)),
            (595, Range::between(2, 11)),
            (547, Range::between(11, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        348,
        4,
        vec![
            (619, Range::higher_than(2)),
            (484, Range::between(7, 11)),
            (627, Range::between(11, 19)),
            (593, Range::between(0, 13)),
            (491, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        348,
        3,
        vec![
            (477, Range::between(1, 13)),
            (608, Range::any()),
            (385, Range::higher_than(5)),
            (400, Range::any()),
            (613, Range::higher_than(10)),
        ],
    );
    dependency_provider.add_dependencies(348, 2, vec![(484, Range::between(0, 5))]);
    dependency_provider.add_dependencies(
        346,
        19,
        vec![
            (351, Range::between(0, 7)),
            (349, Range::between(4, 9)),
            (450, Range::between(16, 17)),
        ],
    );
    dependency_provider.add_dependencies(346, 18, vec![(666, Range::any())]);
    dependency_provider.add_dependencies(
        346,
        16,
        vec![
            (455, Range::between(8, 17)),
            (660, Range::between(3, 10)),
            (418, Range::between(3, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        346,
        14,
        vec![(500, Range::between(5, 13)), (589, Range::between(7, 11))],
    );
    dependency_provider.add_dependencies(
        346,
        13,
        vec![
            (352, Range::between(0, 16)),
            (600, Range::between(9, 10)),
            (437, Range::between(0, 12)),
            (662, Range::between(0, 18)),
            (448, Range::between(16, 17)),
            (635, Range::higher_than(5)),
        ],
    );
    dependency_provider.add_dependencies(
        346,
        12,
        vec![(505, Range::between(13, 18)), (627, Range::between(7, 15))],
    );
    dependency_provider.add_dependencies(346, 10, vec![]);
    dependency_provider.add_dependencies(
        346,
        6,
        vec![
            (635, Range::higher_than(17)),
            (349, Range::between(1, 5)),
            (608, Range::between(6, 13)),
            (599, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(
        346,
        4,
        vec![
            (484, Range::between(2, 17)),
            (608, Range::any()),
            (535, Range::between(6, 13)),
            (495, Range::between(4, 10)),
            (455, Range::between(4, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        346,
        2,
        vec![
            (396, Range::between(9, 16)),
            (455, Range::between(8, 10)),
            (484, Range::between(10, 17)),
            (535, Range::between(13, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        346,
        1,
        vec![
            (560, Range::between(4, 5)),
            (662, Range::between(15, 17)),
            (574, Range::between(10, 15)),
            (400, Range::between(4, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        19,
        vec![
            (662, Range::between(7, 19)),
            (437, Range::any()),
            (601, Range::between(6, 13)),
            (523, Range::higher_than(10)),
            (541, Range::higher_than(8)),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        18,
        vec![
            (491, Range::higher_than(6)),
            (349, Range::between(1, 8)),
            (627, Range::between(11, 12)),
            (448, Range::higher_than(13)),
            (410, Range::between(0, 14)),
        ],
    );
    dependency_provider.add_dependencies(345, 17, vec![(364, Range::between(5, 13))]);
    dependency_provider.add_dependencies(
        345,
        14,
        vec![
            (346, Range::between(2, 13)),
            (445, Range::any()),
            (484, Range::higher_than(16)),
        ],
    );
    dependency_provider.add_dependencies(345, 13, vec![(562, Range::between(0, 18))]);
    dependency_provider.add_dependencies(345, 11, vec![]);
    dependency_provider.add_dependencies(
        345,
        10,
        vec![
            (396, Range::between(3, 8)),
            (562, Range::between(11, 12)),
            (455, Range::between(10, 17)),
            (523, Range::between(0, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        9,
        vec![(447, Range::between(8, 12)), (595, Range::between(10, 16))],
    );
    dependency_provider.add_dependencies(
        345,
        7,
        vec![(535, Range::between(6, 14)), (471, Range::higher_than(2))],
    );
    dependency_provider.add_dependencies(
        345,
        6,
        vec![
            (455, Range::between(7, 18)),
            (569, Range::between(13, 17)),
            (495, Range::between(0, 10)),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        4,
        vec![
            (523, Range::between(1, 2)),
            (462, Range::between(3, 5)),
            (405, Range::between(10, 14)),
            (418, Range::between(0, 9)),
            (535, Range::higher_than(5)),
            (595, Range::between(10, 16)),
            (349, Range::between(10, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        3,
        vec![
            (450, Range::between(0, 17)),
            (405, Range::between(4, 11)),
            (541, Range::between(8, 15)),
            (475, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        1,
        vec![
            (349, Range::between(5, 11)),
            (569, Range::between(8, 19)),
            (491, Range::between(0, 4)),
            (371, Range::between(0, 12)),
            (627, Range::between(12, 18)),
            (364, Range::between(5, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        345,
        0,
        vec![
            (441, Range::higher_than(8)),
            (601, Range::between(0, 5)),
            (635, Range::higher_than(1)),
        ],
    );
    dependency_provider.add_dependencies(
        344,
        19,
        vec![(625, Range::between(6, 9)), (352, Range::between(9, 16))],
    );
    dependency_provider.add_dependencies(
        344,
        15,
        vec![(437, Range::higher_than(15)), (349, Range::between(7, 13))],
    );
    dependency_provider.add_dependencies(
        344,
        14,
        vec![
            (348, Range::between(0, 12)),
            (599, Range::between(7, 14)),
            (443, Range::higher_than(12)),
            (441, Range::between(0, 9)),
            (523, Range::between(10, 12)),
            (462, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(
        344,
        13,
        vec![
            (345, Range::between(4, 12)),
            (594, Range::between(4, 12)),
            (560, Range::between(4, 9)),
            (475, Range::higher_than(3)),
            (352, Range::between(0, 10)),
        ],
    );
    dependency_provider.add_dependencies(344, 9, vec![(450, Range::between(13, 17))]);
    dependency_provider.add_dependencies(
        344,
        6,
        vec![(599, Range::between(7, 15)), (364, Range::between(5, 13))],
    );
    dependency_provider.add_dependencies(
        344,
        3,
        vec![(405, Range::between(1, 14)), (574, Range::between(8, 11))],
    );
    dependency_provider.add_dependencies(
        344,
        2,
        vec![
            (396, Range::between(14, 19)),
            (576, Range::between(8, 11)),
            (647, Range::between(0, 4)),
            (477, Range::higher_than(4)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        17,
        vec![
            (569, Range::higher_than(13)),
            (477, Range::between(4, 16)),
            (396, Range::between(4, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        16,
        vec![
            (535, Range::higher_than(16)),
            (547, Range::between(0, 19)),
            (601, Range::higher_than(16)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        14,
        vec![
            (600, Range::between(12, 13)),
            (448, Range::between(13, 16)),
            (348, Range::between(4, 16)),
            (445, Range::higher_than(8)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        11,
        vec![
            (541, Range::between(7, 8)),
            (364, Range::between(4, 8)),
            (348, Range::between(0, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        10,
        vec![(576, Range::between(0, 16)), (505, Range::higher_than(2))],
    );
    dependency_provider.add_dependencies(
        341,
        7,
        vec![(627, Range::between(7, 12)), (447, Range::higher_than(11))],
    );
    dependency_provider.add_dependencies(
        341,
        6,
        vec![
            (348, Range::between(10, 13)),
            (484, Range::higher_than(14)),
            (494, Range::higher_than(5)),
            (595, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        2,
        vec![
            (364, Range::between(9, 12)),
            (450, Range::between(6, 15)),
            (346, Range::between(0, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        341,
        1,
        vec![
            (479, Range::any()),
            (348, Range::between(16, 18)),
            (574, Range::between(4, 11)),
            (593, Range::higher_than(11)),
            (370, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        19,
        vec![
            (574, Range::higher_than(14)),
            (345, Range::higher_than(9)),
            (344, Range::between(13, 16)),
            (547, Range::higher_than(17)),
            (584, Range::higher_than(3)),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        18,
        vec![
            (584, Range::between(0, 12)),
            (471, Range::between(0, 2)),
            (589, Range::between(8, 9)),
            (345, Range::between(10, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        17,
        vec![
            (645, Range::higher_than(5)),
            (396, Range::between(10, 18)),
            (574, Range::between(11, 19)),
            (418, Range::between(3, 19)),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        16,
        vec![
            (471, Range::between(0, 3)),
            (627, Range::between(7, 12)),
            (405, Range::higher_than(7)),
        ],
    );
    dependency_provider.add_dependencies(335, 15, vec![(462, Range::higher_than(4))]);
    dependency_provider.add_dependencies(335, 12, vec![(445, Range::between(0, 9))]);
    dependency_provider.add_dependencies(
        335,
        11,
        vec![
            (547, Range::between(3, 6)),
            (600, Range::between(0, 9)),
            (627, Range::higher_than(7)),
        ],
    );
    dependency_provider.add_dependencies(335, 8, vec![(599, Range::between(7, 14))]);
    dependency_provider.add_dependencies(
        335,
        7,
        vec![(455, Range::higher_than(17)), (477, Range::between(13, 14))],
    );
    dependency_provider.add_dependencies(
        335,
        6,
        vec![
            (345, Range::between(3, 15)),
            (396, Range::between(0, 4)),
            (505, Range::between(0, 7)),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        3,
        vec![
            (443, Range::higher_than(12)),
            (455, Range::between(4, 9)),
            (346, Range::between(12, 15)),
            (523, Range::between(0, 9)),
            (475, Range::higher_than(9)),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        2,
        vec![
            (660, Range::between(0, 4)),
            (625, Range::between(0, 9)),
            (448, Range::between(11, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        335,
        1,
        vec![
            (569, Range::between(16, 19)),
            (495, Range::between(3, 13)),
            (455, Range::between(0, 18)),
        ],
    );
    dependency_provider.add_dependencies(335, 0, vec![(601, Range::higher_than(1))]);
    dependency_provider.add_dependencies(334, 19, vec![(647, Range::higher_than(11))]);
    dependency_provider.add_dependencies(
        334,
        15,
        vec![
            (662, Range::between(15, 17)),
            (601, Range::between(6, 18)),
            (364, Range::between(4, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        11,
        vec![
            (660, Range::between(0, 2)),
            (349, Range::between(1, 12)),
            (491, Range::between(5, 7)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        8,
        vec![
            (627, Range::higher_than(12)),
            (491, Range::between(0, 4)),
            (484, Range::between(11, 15)),
            (437, Range::between(0, 16)),
            (348, Range::higher_than(4)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        7,
        vec![
            (593, Range::any()),
            (405, Range::between(2, 7)),
            (662, Range::between(0, 4)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        6,
        vec![
            (410, Range::between(5, 14)),
            (462, Range::between(2, 5)),
            (418, Range::between(0, 4)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        5,
        vec![(418, Range::any()), (455, Range::between(4, 10))],
    );
    dependency_provider.add_dependencies(
        334,
        3,
        vec![
            (569, Range::between(12, 19)),
            (535, Range::between(5, 16)),
            (450, Range::between(7, 15)),
            (645, Range::between(2, 7)),
            (477, Range::between(1, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        1,
        vec![
            (484, Range::higher_than(14)),
            (491, Range::higher_than(18)),
            (396, Range::between(17, 19)),
            (541, Range::between(6, 15)),
            (477, Range::between(1, 16)),
            (662, Range::between(3, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        334,
        0,
        vec![
            (405, Range::between(0, 10)),
            (599, Range::between(12, 15)),
            (613, Range::higher_than(16)),
        ],
    );
    dependency_provider.add_dependencies(
        328,
        18,
        vec![
            (396, Range::between(10, 12)),
            (601, Range::between(5, 18)),
            (535, Range::higher_than(13)),
            (594, Range::between(0, 6)),
            (584, Range::higher_than(15)),
            (455, Range::between(8, 11)),
            (569, Range::between(8, 10)),
            (450, Range::between(3, 4)),
            (613, Range::between(8, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        328,
        15,
        vec![
            (396, Range::between(7, 15)),
            (455, Range::between(12, 17)),
            (589, Range::between(6, 8)),
            (348, Range::between(3, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        328,
        13,
        vec![
            (627, Range::any()),
            (441, Range::between(13, 16)),
            (410, Range::between(5, 10)),
            (595, Range::between(10, 16)),
        ],
    );
    dependency_provider.add_dependencies(328, 12, vec![]);
    dependency_provider.add_dependencies(
        328,
        9,
        vec![
            (345, Range::between(0, 4)),
            (600, Range::between(6, 8)),
            (450, Range::higher_than(17)),
            (535, Range::between(8, 13)),
            (448, Range::higher_than(11)),
        ],
    );
    dependency_provider.add_dependencies(328, 5, vec![(396, Range::between(7, 19))]);
    dependency_provider.add_dependencies(
        328,
        1,
        vec![(650, Range::between(0, 3)), (334, Range::between(3, 9))],
    );
    dependency_provider.add_dependencies(
        328,
        0,
        vec![(541, Range::between(6, 17)), (352, Range::higher_than(15))],
    );
    dependency_provider.add_dependencies(
        316,
        19,
        vec![
            (341, Range::between(6, 11)),
            (495, Range::between(11, 13)),
            (370, Range::any()),
            (600, Range::higher_than(6)),
            (371, Range::between(12, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        316,
        17,
        vec![
            (335, Range::higher_than(2)),
            (541, Range::higher_than(13)),
            (344, Range::between(6, 10)),
        ],
    );
    dependency_provider.add_dependencies(
        316,
        16,
        vec![
            (484, Range::higher_than(11)),
            (495, Range::between(5, 12)),
            (505, Range::between(6, 10)),
        ],
    );
    dependency_provider.add_dependencies(316, 14, vec![(662, Range::higher_than(17))]);
    dependency_provider.add_dependencies(
        316,
        13,
        vec![
            (450, Range::between(6, 15)),
            (494, Range::between(0, 6)),
            (410, Range::between(0, 10)),
            (396, Range::between(15, 18)),
            (346, Range::between(2, 17)),
            (475, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        316,
        10,
        vec![
            (627, Range::between(11, 19)),
            (500, Range::between(5, 13)),
            (349, Range::between(6, 8)),
            (505, Range::between(10, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        316,
        6,
        vec![(450, Range::higher_than(6)), (635, Range::between(5, 15))],
    );
    dependency_provider.add_dependencies(
        316,
        3,
        vec![(619, Range::between(2, 9)), (662, Range::between(17, 19))],
    );
    dependency_provider.add_dependencies(
        316,
        2,
        vec![
            (505, Range::between(2, 3)),
            (448, Range::higher_than(11)),
            (574, Range::between(14, 19)),
            (601, Range::between(6, 7)),
            (625, Range::higher_than(8)),
            (396, Range::between(5, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        16,
        vec![
            (352, Range::higher_than(14)),
            (484, Range::between(4, 15)),
            (599, Range::between(6, 13)),
            (462, Range::between(2, 4)),
            (562, Range::between(10, 13)),
            (335, Range::higher_than(12)),
            (589, Range::between(0, 11)),
            (619, Range::between(10, 15)),
        ],
    );
    dependency_provider.add_dependencies(312, 15, vec![(352, Range::between(10, 15))]);
    dependency_provider.add_dependencies(
        312,
        13,
        vec![
            (484, Range::between(10, 15)),
            (594, Range::between(0, 2)),
            (619, Range::between(8, 15)),
            (348, Range::between(4, 17)),
            (345, Range::between(4, 11)),
            (562, Range::between(0, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        12,
        vec![(443, Range::higher_than(12)), (341, Range::higher_than(6))],
    );
    dependency_provider.add_dependencies(
        312,
        11,
        vec![
            (523, Range::between(9, 11)),
            (316, Range::between(0, 3)),
            (608, Range::higher_than(17)),
            (560, Range::between(0, 9)),
            (593, Range::between(8, 12)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        10,
        vec![(601, Range::between(6, 9)), (448, Range::higher_than(13))],
    );
    dependency_provider.add_dependencies(
        312,
        9,
        vec![
            (547, Range::between(3, 18)),
            (662, Range::between(6, 18)),
            (559, Range::between(8, 10)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        6,
        vec![
            (523, Range::between(1, 19)),
            (491, Range::between(3, 6)),
            (445, Range::between(5, 9)),
            (599, Range::between(1, 3)),
            (635, Range::between(1, 18)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        4,
        vec![
            (584, Range::between(11, 12)),
            (547, Range::between(0, 14)),
            (500, Range::between(12, 13)),
            (606, Range::higher_than(19)),
            (335, Range::between(0, 9)),
            (619, Range::between(14, 15)),
            (594, Range::higher_than(4)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        3,
        vec![
            (348, Range::between(8, 14)),
            (627, Range::between(11, 18)),
            (601, Range::higher_than(19)),
            (535, Range::between(5, 6)),
            (589, Range::between(7, 11)),
            (569, Range::between(6, 10)),
            (599, Range::between(1, 15)),
            (455, Range::higher_than(8)),
            (541, Range::between(0, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        312,
        2,
        vec![
            (599, Range::between(1, 2)),
            (627, Range::between(14, 16)),
            (523, Range::between(13, 15)),
            (462, Range::between(3, 7)),
            (594, Range::between(11, 12)),
            (560, Range::between(0, 9)),
            (593, Range::between(11, 13)),
        ],
    );
    dependency_provider.add_dependencies(312, 1, vec![(448, Range::between(0, 17))]);
    dependency_provider.add_dependencies(
        312,
        0,
        vec![(326, Range::higher_than(16)), (505, Range::between(1, 18))],
    );
    dependency_provider.add_dependencies(
        293,
        12,
        vec![
            (334, Range::between(3, 16)),
            (326, Range::between(0, 16)),
            (455, Range::between(8, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        293,
        11,
        vec![
            (662, Range::any()),
            (364, Range::between(12, 13)),
            (443, Range::between(12, 15)),
        ],
    );
    dependency_provider.add_dependencies(
        293,
        10,
        vec![
            (328, Range::between(13, 14)),
            (574, Range::between(4, 11)),
            (450, Range::between(6, 8)),
        ],
    );
    dependency_provider.add_dependencies(293, 7, vec![(593, Range::between(4, 9))]);
    dependency_provider.add_dependencies(
        293,
        3,
        vec![
            (600, Range::between(6, 13)),
            (344, Range::higher_than(19)),
            (443, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        19,
        vec![
            (396, Range::between(5, 19)),
            (328, Range::higher_than(9)),
            (562, Range::between(4, 9)),
        ],
    );
    dependency_provider.add_dependencies(287, 16, vec![(562, Range::between(4, 9))]);
    dependency_provider.add_dependencies(
        287,
        14,
        vec![
            (593, Range::between(8, 12)),
            (562, Range::higher_than(18)),
            (410, Range::between(7, 17)),
            (341, Range::between(6, 12)),
            (335, Range::between(1, 7)),
            (334, Range::between(3, 8)),
            (535, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        13,
        vec![
            (316, Range::between(0, 4)),
            (396, Range::between(9, 15)),
            (334, Range::between(6, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        12,
        vec![
            (349, Range::between(0, 13)),
            (541, Range::higher_than(15)),
            (462, Range::higher_than(2)),
            (335, Range::between(1, 3)),
            (562, Range::higher_than(11)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        11,
        vec![
            (341, Range::between(0, 8)),
            (346, Range::between(2, 14)),
            (312, Range::between(10, 11)),
            (569, Range::between(0, 19)),
            (385, Range::higher_than(19)),
            (371, Range::between(12, 19)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        9,
        vec![
            (484, Range::between(17, 18)),
            (584, Range::higher_than(11)),
            (477, Range::between(1, 13)),
            (601, Range::between(16, 18)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        8,
        vec![
            (341, Range::between(0, 11)),
            (523, Range::between(0, 1)),
            (349, Range::between(0, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        6,
        vec![
            (589, Range::between(6, 11)),
            (441, Range::between(0, 14)),
            (364, Range::higher_than(4)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        5,
        vec![
            (495, Range::higher_than(5)),
            (595, Range::any()),
            (662, Range::between(17, 19)),
            (335, Range::higher_than(6)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        3,
        vec![(418, Range::between(0, 1)), (328, Range::higher_than(9))],
    );
    dependency_provider.add_dependencies(287, 2, vec![(312, Range::between(9, 11))]);
    dependency_provider.add_dependencies(
        287,
        1,
        vec![
            (589, Range::between(1, 7)),
            (505, Range::between(6, 14)),
            (437, Range::any()),
            (500, Range::between(0, 3)),
        ],
    );
    dependency_provider.add_dependencies(
        287,
        0,
        vec![(400, Range::higher_than(4)), (576, Range::between(16, 18))],
    );
    dependency_provider.add_dependencies(
        265,
        19,
        vec![(462, Range::between(12, 13)), (396, Range::between(4, 16))],
    );
    dependency_provider.add_dependencies(
        265,
        18,
        vec![
            (345, Range::between(1, 11)),
            (335, Range::between(6, 19)),
            (574, Range::between(1, 9)),
            (349, Range::between(0, 7)),
        ],
    );
    dependency_provider.add_dependencies(
        265,
        12,
        vec![
            (593, Range::between(11, 14)),
            (601, Range::higher_than(15)),
            (352, Range::between(10, 15)),
            (619, Range::between(0, 9)),
        ],
    );
    dependency_provider.add_dependencies(265, 11, vec![(316, Range::between(6, 11))]);
    dependency_provider.add_dependencies(
        265,
        9,
        vec![
            (448, Range::between(15, 16)),
            (523, Range::between(10, 19)),
            (562, Range::between(4, 13)),
            (601, Range::between(4, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        265,
        7,
        vec![
            (613, Range::between(0, 4)),
            (535, Range::between(5, 16)),
            (650, Range::between(0, 8)),
            (455, Range::between(0, 9)),
            (619, Range::between(2, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        265,
        6,
        vec![
            (455, Range::between(15, 16)),
            (405, Range::between(7, 11)),
            (576, Range::between(0, 16)),
            (462, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(
        259,
        12,
        vec![
            (601, Range::higher_than(12)),
            (352, Range::between(10, 15)),
            (541, Range::between(15, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        259,
        11,
        vec![
            (448, Range::between(11, 17)),
            (345, Range::between(1, 4)),
            (341, Range::between(7, 15)),
            (405, Range::between(0, 15)),
            (593, Range::higher_than(4)),
        ],
    );
    dependency_provider.add_dependencies(259, 10, vec![(662, Range::between(7, 9))]);
    dependency_provider.add_dependencies(
        259,
        5,
        vec![(262, Range::higher_than(5)), (287, Range::higher_than(1))],
    );
    dependency_provider.add_dependencies(
        259,
        0,
        vec![
            (341, Range::between(0, 2)),
            (348, Range::between(12, 18)),
            (601, Range::between(4, 7)),
            (600, Range::higher_than(9)),
            (535, Range::between(5, 13)),
            (593, Range::between(4, 9)),
            (349, Range::higher_than(1)),
        ],
    );
    dependency_provider.add_dependencies(
        255,
        16,
        vec![
            (349, Range::higher_than(4)),
            (625, Range::higher_than(6)),
            (348, Range::between(13, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        255,
        14,
        vec![
            (541, Range::between(7, 9)),
            (348, Range::higher_than(9)),
            (370, Range::any()),
            (265, Range::between(12, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        255,
        13,
        vec![
            (645, Range::between(16, 17)),
            (405, Range::between(9, 10)),
            (574, Range::between(0, 19)),
            (608, Range::higher_than(12)),
        ],
    );
    dependency_provider.add_dependencies(
        255,
        11,
        vec![
            (448, Range::between(0, 13)),
            (462, Range::between(3, 8)),
            (535, Range::between(12, 14)),
            (349, Range::between(11, 13)),
        ],
    );
    dependency_provider.add_dependencies(255, 8, vec![(312, Range::between(2, 14))]);
    dependency_provider.add_dependencies(
        255,
        5,
        vec![
            (348, Range::between(4, 5)),
            (535, Range::between(0, 7)),
            (396, Range::higher_than(7)),
            (441, Range::between(8, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        19,
        vec![
            (335, Range::between(12, 16)),
            (349, Range::between(1, 6)),
            (405, Range::higher_than(10)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        18,
        vec![
            (312, Range::between(2, 14)),
            (600, Range::higher_than(9)),
            (265, Range::between(0, 10)),
            (593, Range::between(8, 10)),
            (491, Range::between(2, 4)),
            (608, Range::between(0, 10)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        17,
        vec![
            (341, Range::between(2, 7)),
            (574, Range::between(9, 11)),
            (450, Range::higher_than(16)),
            (479, Range::any()),
            (560, Range::between(0, 3)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        15,
        vec![
            (352, Range::higher_than(17)),
            (484, Range::higher_than(11)),
            (627, Range::between(12, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        14,
        vec![
            (523, Range::between(0, 7)),
            (287, Range::between(8, 10)),
            (477, Range::between(7, 8)),
            (462, Range::between(2, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        12,
        vec![(335, Range::between(1, 16)), (293, Range::between(7, 11))],
    );
    dependency_provider.add_dependencies(
        250,
        11,
        vec![
            (595, Range::between(10, 11)),
            (523, Range::between(6, 10)),
            (445, Range::between(0, 6)),
            (410, Range::between(9, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        10,
        vec![
            (370, Range::any()),
            (334, Range::between(0, 16)),
            (297, Range::higher_than(17)),
            (287, Range::between(0, 10)),
            (364, Range::between(0, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        9,
        vec![
            (348, Range::between(0, 4)),
            (334, Range::between(6, 12)),
            (650, Range::between(2, 8)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        7,
        vec![(660, Range::higher_than(3)), (600, Range::between(6, 10))],
    );
    dependency_provider.add_dependencies(
        250,
        5,
        vec![
            (547, Range::between(5, 12)),
            (495, Range::between(3, 10)),
            (660, Range::between(0, 2)),
            (349, Range::between(1, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        4,
        vec![
            (471, Range::between(0, 3)),
            (660, Range::higher_than(9)),
            (462, Range::between(14, 15)),
            (559, Range::higher_than(10)),
            (560, Range::higher_than(4)),
            (584, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        250,
        2,
        vec![
            (595, Range::between(2, 15)),
            (455, Range::between(10, 18)),
            (448, Range::between(0, 13)),
            (345, Range::between(7, 10)),
            (450, Range::between(10, 18)),
            (441, Range::between(0, 16)),
            (505, Range::between(13, 18)),
            (613, Range::higher_than(10)),
            (495, Range::between(3, 9)),
        ],
    );
    dependency_provider.add_dependencies(
        249,
        15,
        vec![
            (547, Range::between(0, 3)),
            (312, Range::between(11, 13)),
            (448, Range::higher_than(12)),
            (613, Range::between(0, 9)),
            (250, Range::between(4, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        249,
        10,
        vec![
            (341, Range::between(6, 11)),
            (349, Range::higher_than(6)),
            (645, Range::between(2, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        249,
        9,
        vec![
            (560, Range::between(4, 7)),
            (251, Range::any()),
            (523, Range::between(0, 11)),
            (477, Range::higher_than(13)),
        ],
    );
    dependency_provider.add_dependencies(
        245,
        5,
        vec![
            (495, Range::between(8, 12)),
            (576, Range::higher_than(8)),
            (450, Range::between(10, 14)),
            (660, Range::between(0, 10)),
        ],
    );
    dependency_provider.add_dependencies(
        245,
        3,
        vec![
            (364, Range::between(12, 13)),
            (335, Range::between(0, 7)),
            (293, Range::between(0, 8)),
            (569, Range::between(8, 13)),
            (547, Range::between(12, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        245,
        2,
        vec![(541, Range::between(7, 16)), (268, Range::between(9, 14))],
    );
    dependency_provider.add_dependencies(
        245,
        1,
        vec![(613, Range::between(0, 9)), (334, Range::between(5, 9))],
    );
    dependency_provider.add_dependencies(
        228,
        17,
        vec![
            (341, Range::between(0, 15)),
            (559, Range::higher_than(11)),
            (535, Range::between(12, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        228,
        15,
        vec![
            (265, Range::between(0, 13)),
            (250, Range::between(4, 18)),
            (606, Range::between(7, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        228,
        14,
        vec![(491, Range::any()), (371, Range::between(0, 12))],
    );
    dependency_provider.add_dependencies(
        228,
        7,
        vec![
            (595, Range::between(10, 15)),
            (523, Range::higher_than(9)),
            (662, Range::between(16, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        228,
        6,
        vec![
            (608, Range::higher_than(6)),
            (293, Range::between(7, 11)),
            (559, Range::higher_than(8)),
            (341, Range::between(0, 2)),
            (613, Range::between(8, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        18,
        vec![
            (660, Range::any()),
            (312, Range::higher_than(15)),
            (584, Range::higher_than(3)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        17,
        vec![
            (455, Range::between(4, 13)),
            (562, Range::between(4, 13)),
            (348, Range::between(9, 15)),
            (495, Range::between(3, 9)),
            (547, Range::higher_than(6)),
            (312, Range::between(6, 12)),
            (371, Range::between(12, 19)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        16,
        vec![(255, Range::between(11, 14)), (589, Range::higher_than(1))],
    );
    dependency_provider.add_dependencies(
        227,
        15,
        vec![(523, Range::between(11, 14)), (450, Range::between(0, 4))],
    );
    dependency_provider.add_dependencies(
        227,
        14,
        vec![(455, Range::between(9, 18)), (625, Range::between(8, 9))],
    );
    dependency_provider.add_dependencies(
        227,
        10,
        vec![
            (242, Range::higher_than(4)),
            (287, Range::between(1, 7)),
            (650, Range::higher_than(14)),
            (230, Range::between(7, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        9,
        vec![
            (410, Range::between(5, 8)),
            (574, Range::between(2, 15)),
            (351, Range::between(0, 7)),
            (334, Range::higher_than(11)),
            (599, Range::between(2, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        8,
        vec![
            (495, Range::between(0, 5)),
            (660, Range::between(0, 10)),
            (462, Range::between(7, 8)),
            (627, Range::between(0, 2)),
            (613, Range::between(0, 4)),
            (328, Range::between(13, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        6,
        vec![(625, Range::between(0, 7)), (448, Range::between(0, 13))],
    );
    dependency_provider.add_dependencies(
        227,
        5,
        vec![
            (593, Range::higher_than(4)),
            (265, Range::between(7, 10)),
            (316, Range::between(14, 17)),
            (405, Range::between(2, 8)),
            (471, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        4,
        vec![
            (264, Range::between(0, 4)),
            (479, Range::any()),
            (344, Range::between(6, 7)),
            (562, Range::between(4, 5)),
        ],
    );
    dependency_provider.add_dependencies(
        227,
        3,
        vec![
            (448, Range::between(12, 13)),
            (650, Range::between(0, 15)),
            (559, Range::between(9, 11)),
            (594, Range::between(1, 12)),
            (348, Range::between(0, 3)),
        ],
    );
    dependency_provider.add_dependencies(
        215,
        15,
        vec![(349, Range::between(10, 12)), (619, Range::between(0, 7))],
    );
    dependency_provider.add_dependencies(
        215,
        13,
        vec![(450, Range::between(3, 5)), (662, Range::between(16, 19))],
    );
    dependency_provider.add_dependencies(
        215,
        11,
        vec![
            (601, Range::between(8, 13)),
            (316, Range::between(3, 18)),
            (613, Range::between(8, 11)),
            (505, Range::between(1, 14)),
            (228, Range::between(0, 15)),
            (245, Range::between(0, 4)),
            (559, Range::between(11, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        215,
        8,
        vec![
            (606, Range::any()),
            (245, Range::between(0, 2)),
            (608, Range::between(0, 10)),
            (334, Range::between(0, 6)),
            (450, Range::between(0, 18)),
            (341, Range::between(0, 12)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        19,
        vec![
            (662, Range::between(8, 17)),
            (608, Range::between(6, 7)),
            (627, Range::between(14, 15)),
            (600, Range::between(6, 9)),
            (589, Range::higher_than(7)),
            (316, Range::between(3, 17)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        17,
        vec![
            (576, Range::between(7, 9)),
            (245, Range::between(2, 4)),
            (364, Range::higher_than(9)),
            (250, Range::between(10, 12)),
            (559, Range::between(0, 13)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        13,
        vec![
            (660, Range::between(0, 4)),
            (287, Range::higher_than(14)),
            (328, Range::between(12, 14)),
            (450, Range::between(3, 17)),
            (205, Range::between(2, 13)),
            (491, Range::between(2, 4)),
        ],
    );
    dependency_provider.add_dependencies(190, 12, vec![(645, Range::between(6, 17))]);
    dependency_provider.add_dependencies(
        190,
        11,
        vec![
            (370, Range::any()),
            (210, Range::between(8, 15)),
            (495, Range::between(11, 13)),
            (594, Range::higher_than(11)),
            (345, Range::between(11, 12)),
            (484, Range::between(0, 8)),
            (334, Range::between(5, 6)),
            (349, Range::higher_than(3)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        9,
        vec![
            (547, Range::between(2, 17)),
            (227, Range::between(4, 16)),
            (287, Range::between(1, 12)),
            (341, Range::between(10, 12)),
            (396, Range::between(4, 19)),
            (348, Range::higher_than(7)),
            (523, Range::between(8, 9)),
            (208, Range::any()),
            (199, Range::between(3, 16)),
            (400, Range::between(4, 5)),
            (334, Range::between(0, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        8,
        vec![
            (600, Range::higher_than(6)),
            (560, Range::between(0, 5)),
            (265, Range::between(0, 8)),
            (541, Range::between(0, 7)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        5,
        vec![
            (619, Range::between(4, 7)),
            (250, Range::between(14, 19)),
            (627, Range::higher_than(14)),
        ],
    );
    dependency_provider.add_dependencies(
        190,
        3,
        vec![(441, Range::any()), (400, Range::between(0, 1))],
    );
    dependency_provider.add_dependencies(
        190,
        0,
        vec![
            (265, Range::higher_than(18)),
            (293, Range::between(7, 12)),
            (202, Range::between(9, 10)),
            (535, Range::between(13, 14)),
            (328, Range::between(12, 13)),
            (251, Range::between(0, 18)),
        ],
    );
    dependency_provider.add_dependencies(
        128,
        15,
        vec![
            (205, Range::between(5, 13)),
            (312, Range::between(10, 13)),
            (335, Range::between(2, 7)),
            (589, Range::between(0, 9)),
            (385, Range::between(0, 4)),
            (346, Range::higher_than(10)),
        ],
    );
    dependency_provider.add_dependencies(
        128,
        13,
        vec![
            (190, Range::between(3, 6)),
            (312, Range::between(10, 11)),
            (348, Range::higher_than(10)),
            (495, Range::between(5, 12)),
            (484, Range::between(4, 9)),
            (341, Range::between(0, 7)),
        ],
    );
    dependency_provider.add_dependencies(
        128,
        12,
        vec![
            (484, Range::between(6, 8)),
            (500, Range::between(0, 3)),
            (328, Range::between(0, 16)),
            (250, Range::between(5, 8)),
            (547, Range::between(14, 17)),
            (190, Range::between(3, 14)),
        ],
    );
    dependency_provider.add_dependencies(
        128,
        11,
        vec![
            (606, Range::higher_than(16)),
            (418, Range::between(3, 4)),
            (242, Range::between(5, 10)),
            (595, Range::between(16, 17)),
            (287, Range::between(1, 3)),
            (335, Range::between(2, 4)),
            (349, Range::between(4, 15)),
            (265, Range::between(11, 13)),
            (477, Range::any()),
            (635, Range::between(0, 18)),
            (316, Range::higher_than(14)),
            (190, Range::between(0, 1)),
            (344, Range::higher_than(6)),
        ],
    );
    dependency_provider.add_dependencies(
        128,
        9,
        vec![
            (287, Range::between(12, 13)),
            (541, Range::between(0, 7)),
            (455, Range::between(12, 16)),
            (265, Range::between(0, 8)),
            (259, Range::higher_than(10)),
            (250, Range::between(17, 19)),
            (574, Range::between(10, 15)),
            (505, Range::between(1, 18)),
            (447, Range::any()),
        ],
    );
    dependency_provider.add_dependencies(
        128,
        8,
        vec![
            (349, Range::between(0, 11)),
            (523, Range::between(8, 11)),
            (619, Range::higher_than(2)),
            (595, Range::higher_than(16)),
            (316, Range::between(13, 18)),
            (547, Range::between(13, 18)),
            (410, Range::between(5, 6)),
        ],
    );
    dependency_provider.add_dependencies(128, 5, vec![(541, Range::between(0, 14))]);
    dependency_provider.add_dependencies(
        96,
        14,
        vec![
            (599, Range::between(10, 15)),
            (171, Range::between(0, 3)),
            (255, Range::higher_than(8)),
            (593, Range::higher_than(13)),
            (559, Range::between(0, 9)),
            (242, Range::between(5, 14)),
            (370, Range::higher_than(16)),
            (574, Range::between(0, 2)),
        ],
    );
    dependency_provider.add_dependencies(
        96,
        12,
        vec![
            (600, Range::between(8, 10)),
            (249, Range::any()),
            (128, Range::between(5, 14)),
            (405, Range::between(6, 12)),
            (352, Range::between(10, 17)),
            (595, Range::higher_than(15)),
            (613, Range::between(0, 11)),
        ],
    );
    dependency_provider.add_dependencies(
        96,
        10,
        vec![
            (443, Range::between(0, 15)),
            (341, Range::between(2, 15)),
            (447, Range::higher_than(13)),
            (169, Range::between(9, 11)),
            (500, Range::between(5, 13)),
            (344, Range::between(13, 14)),
            (619, Range::between(4, 15)),
            (259, Range::between(0, 1)),
            (418, Range::between(8, 16)),
        ],
    );
    dependency_provider.add_dependencies(
        13,
        13,
        vec![
            (547, Range::higher_than(18)),
            (505, Range::between(1, 18)),
            (441, Range::between(0, 3)),
            (171, Range::between(2, 12)),
        ],
    );
    dependency_provider.add_dependencies(
        13,
        12,
        vec![
            (208, Range::between(0, 8)),
            (124, Range::between(0, 15)),
            (405, Range::higher_than(2)),
            (100, Range::higher_than(6)),
            (574, Range::between(0, 10)),
            (396, Range::between(9, 12)),
            (287, Range::between(3, 6)),
        ],
    );
    dependency_provider.add_dependencies(
        13,
        10,
        vec![
            (505, Range::between(10, 14)),
            (215, Range::between(11, 14)),
            (227, Range::higher_than(10)),
        ],
    );
    dependency_provider.add_dependencies(
        0,
        0,
        vec![
            (13, Range::between(10, 13)),
            (475, Range::between(3, 4)),
            (96, Range::between(10, 15)),
            (344, Range::between(0, 15)),
            (600, Range::any()),
            (479, Range::any()),
            (523, Range::between(0, 10)),
        ],
    );

    // bench
    b.iter(|| {
        let _ = resolve(&dependency_provider, 0, 0);
    });
}
