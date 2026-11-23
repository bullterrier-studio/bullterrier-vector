//! The "looks the same" helpers of `common/looks_the_same.rs`, on known-good and known-bad inputs.

use crate::common::looks_the_same::{pixels_look_the_same, svg_looks_the_same};

/// A 100 by 100 image of one RGBA colour.
fn flat(rgba: [u8; 4]) -> Vec<u8> {
    rgba.repeat(100 * 100)
}

#[test]
fn a_few_samples_one_level_off_look_the_same() {
    let mut other = flat([10, 20, 30, 255]);
    for at in [1, 29, 401] {
        other[at] += 1;
    }
    assert_eq!(pixels_look_the_same(&flat([10, 20, 30, 255]), &other), Ok(()));
}

#[test]
fn one_sample_three_levels_off_is_refused() {
    let mut other = flat([10, 20, 30, 255]);
    other[802] += 3;
    let refused = pixels_look_the_same(&flat([10, 20, 30, 255]), &other);
    assert!(refused.is_err_and(|message| message.contains("largest by 3")));
}

#[test]
fn a_few_samples_two_levels_off_look_the_same_but_many_do_not() {
    let base = flat([10, 20, 30, 255]);
    let mut few = base.clone();
    few[5] += 2;
    assert_eq!(pixels_look_the_same(&base, &few), Ok(()));
    let mut many = base.clone();
    for at in 0..200 {
        many[at * 4 + 1] += 2;
    }
    assert!(pixels_look_the_same(&base, &many).is_err());
}

#[test]
fn images_of_different_lengths_are_refused() {
    assert!(pixels_look_the_same(&flat([1, 2, 3, 4]), &[1, 2, 3, 4]).is_err());
}

#[test]
fn svg_numbers_within_the_tolerance_look_the_same_and_others_do_not() {
    let text = r#"<path d="M10.5 -2 L100.25 3" stroke-width="2"/>"#;
    assert_eq!(
        svg_looks_the_same(text, r#"<path d="M10.503 -2 L100.25 3.003" stroke-width="2"/>"#),
        Ok(())
    );
    assert!(svg_looks_the_same(text, r#"<path d="M10.51 -2 L100.25 3" stroke-width="2"/>"#).is_err());
    assert!(svg_looks_the_same(text, r#"<path d="M10.5 -2 L100.25 3 Z" stroke-width="2"/>"#).is_err());
}
