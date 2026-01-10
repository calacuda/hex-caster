// https://depts.washington.edu/acelab/proj/dollar/dollar.pdf

use core::f32::consts::{PHI, PI};

use alloc::{borrow::ToOwned, vec::Vec};
use log::*;
use num_traits::Float;

use crate::{Spell, SpellId};

pub type NormedPoint = (f32, f32);
pub type NormedSpell = Vec<(f32, f32)>;

pub const THETA: f32 = PI / 4.0;
pub const NEG_THETA: f32 = -1.0 * THETA;
pub const THETA_DELTA: f32 = PI / 90.0;
const N: usize = 64;
const SIZE: f32 = 256.0;

// async fn lerp_2d(p_1: (f32, f32), p_2: (f32, f32), fract: f32) -> (f32, f32) {
//     let lerp = |start, end, t| start + t * (end - start);
//
//     let x = lerp(p_1.0, p_1.1, fract);
//     let x = if x <= 1.0 { x } else { 1.0 };
//
//     let y = lerp(p_2.0, p_2.1, fract);
//     let y = if y <= 1.0 { y } else { 1.0 };
//
//     (x, y)
// }

// STEP 1

async fn resample(points: &NormedSpell, n: usize) -> NormedSpell {
    let mut loc_points = points.clone();
    let cap_i = path_length(&loc_points).await / (n - 1) as f32;
    let mut cap_d = 0.0;
    let mut new_points = Vec::with_capacity(n);
    new_points.push(loc_points[0]);

    for i in 1..loc_points.len() {
        let a = loc_points[i - 1];
        let b = loc_points[i];
        let d = distance(a, b).await;

        if (cap_d + d) >= cap_i {
            let qx = a.0 + ((cap_i - cap_d) / d) * (b.0 - a.0);
            let qy = a.1 + ((cap_i - cap_d) / d) * (b.1 - a.1);
            let q = (qx, qy);
            new_points.push(q);
            loc_points.insert(i + 1, q);

            cap_d = 0.0;
        } else {
            cap_d += d;
        }
    }

    if new_points.len() > n {
        error!("new_point.len() = {}", new_points.len());
    }

    while new_points.len() < n {
        new_points.push(loc_points.last().unwrap().to_owned());
    }

    new_points
}

async fn path_length(points: &NormedSpell) -> f32 {
    let mut d = 0.0;

    for i in 1..points.len() {
        d += distance(points[i - 1], points[i]).await;
    }

    d
}

async fn distance(a: NormedPoint, b: NormedPoint) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

// STEP 2

async fn rotate_by(points: &NormedSpell, angle: f32) -> NormedSpell {
    let c = centroid(points).await;
    // info!("rotate_by -> centroid: {c:?}");
    let cos = angle.cos();
    let sin = angle.sin();
    // info!("points cloned");

    points
        .into_iter()
        .map(|p| {
            let qx = (p.0 - c.0) * cos - (p.1 - c.1) * sin + c.0;
            let qy = (p.0 - c.0) * sin + (p.1 - c.1) * cos + c.1;
            // p.0 = qx;
            // p.1 = qy;
            (qx, qy)
        })
        .collect()
}

// STEP 3

async fn scale_to(points: &NormedSpell, size: f32) -> NormedSpell {
    let cap_b = bounding_box(points).await;
    let mut points = points.clone();

    for p in points.iter_mut() {
        p.0 = p.0 * size / cap_b.0;
        p.1 = p.1 * size / cap_b.1;
    }

    points
}

async fn translate_to(points: &NormedSpell, k: NormedPoint) -> NormedSpell {
    let c = centroid(points).await;
    let dx = k.0 - c.0;
    let dy = k.1 - c.1;

    points.into_iter().map(|(x, y)| (x + dx, y + dy)).collect()
}

async fn bounding_box(spell: &NormedSpell) -> (f32, f32) {
    let min_x = spell
        .iter()
        .fold(f32::INFINITY, |acc, (x, _)| if acc > *x { *x } else { acc });
    let min_y = spell
        .iter()
        .fold(f32::INFINITY, |acc, (_, y)| if acc > *y { *y } else { acc });
    info!("min_x: {min_x}, min_y: {min_y}");
    let (w, h) = (
        spell.iter().fold(
            f32::NEG_INFINITY,
            |acc, (x, _)| if acc < *x { *x } else { acc },
        ) - min_x,
        spell.iter().fold(
            f32::NEG_INFINITY,
            |acc, (_, y)| if acc < *y { *y } else { acc },
        ) - min_y,
    );
    info!("w: {w}, h: {h}");

    (w, h)
}

async fn centroid(points: &NormedSpell) -> NormedPoint {
    let n = points.len() as f32;
    let x: f32 = points.iter().map(|(x, _)| *x).sum();
    let y: f32 = points.iter().map(|(_, y)| *y).sum();

    (x / n, y / n)
}

// STEP 4

async fn recognize(
    cast_spell: &NormedSpell,
    templates: &[NormedSpell],
    size: f32,
) -> (SpellId, f32) {
    let mut b = f32::INFINITY;
    let mut best_match = 0;

    for (i, template) in templates.into_iter().enumerate() {
        info!("comparing to template: {i}");
        let d = distance_at_best_angle(cast_spell, template).await;

        if d < b {
            b = d;
            best_match = i;
        }
    }

    info!("calculated partial match score: {b}");

    let score = 1.0 - b / (0.5 * (2.0 * size * size).sqrt());

    (best_match, score)
}

async fn distance_at_best_angle(cast_spell: &NormedSpell, template: &NormedSpell) -> f32 {
    let mut x1 = PHI * NEG_THETA + (1.0 - PHI) * THETA;
    // info!("one");
    let mut f1 = distance_at_angle(cast_spell, template, x1).await;
    let mut x2 = (1.0 - PHI) * NEG_THETA + PHI * THETA;
    // info!("two");
    let mut f2 = distance_at_angle(cast_spell, template, x2).await;
    let mut a = NEG_THETA;
    let mut b = THETA;

    info!("will now loop");

    while (b - a).abs() > THETA_DELTA && (b - a).abs() < f32::INFINITY {
        // debug!(
        //     "(b - a).abs() > THETA_DELTA -> {} > {} = {}",
        //     (b - a).abs(),
        //     THETA_DELTA,
        //     (b - a).abs() > THETA_DELTA
        // );

        if f1 < f2 {
            // debug!("less then");
            b = x2;
            x2 = x1;
            f2 = f1;
            x1 = PHI * a + (1. - PHI) * b;
            f1 = distance_at_angle(cast_spell, template, x1).await;
        } else {
            // debug!("greater then");
            a = x1;
            x1 = x2;
            f1 = f2;
            x2 = (1.0 - PHI) * a + PHI * b;
            f2 = distance_at_angle(cast_spell, template, x2).await;
        }
    }

    debug!(
        "(b - a).abs() > THETA_DELTA -> {} > {} = {}",
        (b - a).abs(),
        THETA_DELTA,
        (b - a).abs() > THETA_DELTA
    );
    debug!("a: {a}, b: {b}");
    info!("distance_at_best_angle calculated");

    f1.min(f2)
}

async fn distance_at_angle(cast_spell: &NormedSpell, template: &NormedSpell, _angle: f32) -> f32 {
    // let new_points = rotate_by(cast_spell, angle).await;
    // let d = path_distance(new_points, template).await;
    // info!("angle: {angle}");
    let d = path_distance(cast_spell.clone(), template).await;

    d
}

async fn path_distance(new_points: NormedSpell, template: &NormedSpell) -> f32 {
    let mut d = 0.0;

    for i in 0..new_points.len() {
        if let (Some(p1), Some(p2)) = (new_points.get(i), template.get(i)) {
            d += distance(*p1, *p2).await;
        } else {
            error!(
                "i value {i} failed as index. new_points.len() = {}, template.len() = {}",
                new_points.len(),
                template.len()
            );
        }
    }

    d / new_points.len() as f32
}

// Entry Points

pub async fn process_stroke(spell: Spell) -> NormedSpell {
    let spell = spell.into_iter().map(|(x, y)| (x as f32, y as f32));
    // Step 1
    let mut points = resample(&spell.collect(), N).await;

    while points.len() != N {
        points = resample(&points, N).await;
    }
    // Step 3 (skipping rotation)
    let points = scale_to(&points, SIZE).await;
    let points = translate_to(&points, (0., 0.)).await;

    points
}

pub async fn spell_compare(cast_spell: NormedSpell, templates: &[NormedSpell]) -> (usize, f32) {
    info!("comparing two spells");
    recognize(&cast_spell, templates, SIZE).await
}
