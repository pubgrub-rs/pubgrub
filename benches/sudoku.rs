//! A sudoku solver.
//!
//! Uses `Arc<usize>` for being closer to real versions.
// SPDX-License-Identifier: MPL-2.0

use pubgrub::{resolve, OfflineDependencyProvider, Range};
use std::fmt;
use std::sync::Arc;
use version_ranges::Ranges;

use criterion::*;

/// The size of a box in the board.
const BOARD_BASE: usize = 3;
/// The size of the board.
const BOARD_SIZE: usize = BOARD_BASE * BOARD_BASE;

type DP = OfflineDependencyProvider<SudokuPackage, Range<Arc<usize>>>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum SudokuPackage {
    /// Add all known fields.
    Root,
    /// Version is the value of the cell.
    Cell { row: usize, col: usize },
}

impl fmt::Display for SudokuPackage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SudokuPackage::Root => f.write_str("root"),
            SudokuPackage::Cell { row, col } => {
                write!(f, "({col}, {row})")
            }
        }
    }
}

fn from_board(b: &str) -> Vec<(SudokuPackage, Range<Arc<usize>>)> {
    let mut out = vec![];
    for (row, line) in b
        .trim()
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('-'))
        .enumerate()
    {
        for (col, val) in line
            .split_ascii_whitespace()
            .filter(|c| !c.starts_with('|'))
            .enumerate()
        {
            if let Some(val) = val.chars().next().unwrap().to_digit(10) {
                out.push((
                    SudokuPackage::Cell {
                        row: row + 1,
                        col: col + 1,
                    },
                    Range::singleton(val as usize),
                ));
            }
        }
    }
    out
}

/// Encode all the exclusions from assigning a cell to a value
fn encode_constraints(
    dependency_provider: &mut OfflineDependencyProvider<SudokuPackage, Ranges<Arc<usize>>>,
) {
    for row in 1..=BOARD_SIZE {
        for col in 1..=BOARD_SIZE {
            for val in 1..=BOARD_SIZE {
                let mut deps = vec![];
                // A number may only occur once in a row
                for row_ in 1..=BOARD_SIZE {
                    if row_ == row {
                        continue;
                    }
                    deps.push((
                        SudokuPackage::Cell { row: row_, col },
                        Range::singleton(Arc::new(val)).complement(),
                    ))
                }
                // A number may only occur once in a col
                for col_ in 1..=BOARD_SIZE {
                    if col_ == col {
                        continue;
                    }
                    deps.push((
                        SudokuPackage::Cell { row, col: col_ },
                        Range::singleton(Arc::new(val)).complement(),
                    ))
                }
                // A number may only occur once in a box
                let box_base_row = row - ((row - 1) % BOARD_BASE);
                let box_base_col = col - ((col - 1) % BOARD_BASE);
                for row_ in box_base_row..box_base_row + BOARD_BASE {
                    for col_ in box_base_col..box_base_col + BOARD_BASE {
                        if col_ == col && row_ == row {
                            continue;
                        }
                        deps.push((
                            SudokuPackage::Cell {
                                row: row_,
                                col: col_,
                            },
                            Range::singleton(Arc::new(val)).complement(),
                        ))
                    }
                }
                let name = SudokuPackage::Cell { row, col };
                dependency_provider.add_dependencies(name, val, deps)
            }
        }
    }
}

fn solve(c: &mut Criterion, board: Vec<(SudokuPackage, Ranges<Arc<usize>>)>, case: &str) {
    let mut dependency_provider = DP::new();
    encode_constraints(&mut dependency_provider);
    dependency_provider.add_dependencies(SudokuPackage::Root, Arc::new(1usize), board);
    c.bench_function(case, |b| {
        b.iter(|| {
            let _ = resolve(&dependency_provider, SudokuPackage::Root, Arc::new(1usize));
        })
    });
}

fn bench_solve(c: &mut Criterion) {
    let easy = from_board(
        r#"
        5 3 _ | _ 7 _ | _ _ _
        6 _ _ | 1 9 5 | _ _ _
        _ 9 8 | _ _ _ | _ 6 _
       -------+-------+-------
        8 5 9 | _ 6 1 | 4 2 3
        4 2 6 | 8 5 3 | 7 9 1
        7 1 3 | 9 2 4 | 8 5 6
       -------+-------+-------
        _ 6 _ | _ _ _ | 2 8 _
        _ _ _ | 4 1 9 | _ _ 5
        _ _ _ | _ 8 6 | 1 7 9"#,
    );
    let hard = from_board(
        r#"
        5 3 _ | _ 7 _ | _ _ _
        6 _ _ | 1 9 5 | _ _ _
        _ 9 8 | _ _ _ | _ 6 _
        -------+-------+-------
        8 _ _ | _ 6 _ | _ _ 3
        4 _ _ | 8 _ 3 | _ _ 1
        7 _ _ | _ 2 _ | _ _ 6
        -------+-------+-------
        _ 6 _ | _ _ _ | 2 8 _
        _ _ _ | 4 1 9 | _ _ 5
        _ _ _ | _ 8 _ | _ 7 9"#,
    );
    solve(c, easy, "sudoku-easy");
    solve(c, hard, "sudoku-hard");
}

criterion_group!(benches, bench_solve);
criterion_main!(benches);
