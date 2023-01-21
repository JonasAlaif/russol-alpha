#![feature(nll)]
#![feature(box_patterns)]
#![feature(box_syntax)]

use russol_contracts::*;

struct Point {
    x: Box<u16>,
    y: Box<u16>,
}

#[requires(u16::MAX - *a >= b)]
#[ensures(*result == *a + b)]
fn add(a: Box<u16>, b: u16) -> Box<u16> {
  Box::new((*a + b) as u16)
}

#[requires(u16::MAX - *p.x >= s)]
#[ensures(*result.x == *p.x + s)]
#[ensures(*result.y == *p.y)]
fn shift_x(p: Point, s: u16) -> Point {
  let x = Box::new((*p.x + s) as u16);
  Point { x, y: p.y }
}
