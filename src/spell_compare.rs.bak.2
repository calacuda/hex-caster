use core::f32::consts::PI;

use alloc::vec::Vec;
use log::*;
use num_traits::Float;

use crate::Spell;

type NormedSpell = Vec<(f32, f32)>;

pub async fn spell_compare(drawn: &Spell, against: &Spell) -> f32 {
    let ((drawn, drawn_ar, drawn_size), (compare, compare_ar, compare_size)) =
        (normalize(drawn).await, normalize(against).await);
    let mk_map_f = |scale| move |(x, y)| (x * scale, y * scale);
    let (_ar, drawn, compare) = if drawn_size > compare_size {
        let size_ratio = compare_size / drawn_size;
        (
            size_ratio,
            drawn.into_iter().map(mk_map_f(size_ratio)).collect(),
            compare,
        )
    } else if drawn_size > compare_size {
        let size_ratio = drawn_size / compare_size;
        (
            size_ratio,
            drawn,
            compare.into_iter().map(mk_map_f(size_ratio)).collect(),
        )
    } else {
        (1.0, drawn, compare)
    };

    info!("about to compare");
    let line_differnece = do_spell_compare(drawn, compare).await;
    info!("line_diff = {line_differnece}");
    let ar_diff = 1.0
        - if compare_ar < drawn_ar {
            compare_ar / drawn_ar
        } else {
            drawn_ar / compare_ar
        };

    info!("ar_diff = {ar_diff}");
    // info!("ar = {ar}");

    line_differnece * 0.75 + ar_diff * 0.25
    // (line_differnece * 0.8 + ar_diff * 0.2) * ar

    // line_differnece + ar
    // line_differnece
}

pub async fn maybe_spell_compare(drawn: &Spell, against: &Spell) -> Option<f32> {
    let ((drawn, drawn_ar, drawn_size), (compare, compare_ar, compare_size)) =
        (normalize(drawn).await, normalize(against).await);
    let mk_map_f = |scale| move |(x, y)| (x * scale, y * scale);
    let (ar, drawn, compare) = if drawn_size > compare_size {
        let size_ratio = compare_size / drawn_size;
        (
            size_ratio,
            drawn.into_iter().map(mk_map_f(size_ratio)).collect(),
            compare,
        )
    } else if drawn_size > compare_size {
        let size_ratio = drawn_size / compare_size;
        (
            size_ratio,
            drawn,
            compare.into_iter().map(mk_map_f(size_ratio)).collect(),
        )
    } else {
        (1.0, drawn, compare)
    };

    info!("about to compare");
    let line_differnece = do_spell_compare(drawn, compare).await;
    info!("line_diff = {line_differnece}");
    let ar_diff = 1.0
        - if compare_ar < drawn_ar {
            compare_ar / drawn_ar
        } else {
            drawn_ar / compare_ar
        };

    info!("ar_diff = {ar_diff}");

    // let comp = line_differnece * 0.75 + ar_diff * 0.25;
    // let comp = line_differnece * 0.8 + ar * 0.2;
    // let comp = line_differnece * 0.85 + ar * 0.15;
    // let comp = line_differnece * 0.9 + ar * 0.1;
    let comp = line_differnece;

    if !comp.is_nan() { Some(comp) } else { None }
}

pub async fn normalize(spell: &Spell) -> (NormedSpell, f32, f32) {
    let min_x = spell.iter().min_by_key(|(x, _y)| x).unwrap_or(&(0, 0)).0 as f32;
    let min_y = spell.iter().min_by_key(|(_x, y)| y).unwrap_or(&(0, 0)).1 as f32;
    info!("min_x: {min_x}, min_y: {min_y}");
    let (w, h) = (
        spell
            .iter()
            .max_by_key(|(x, _y)| x)
            .unwrap_or(&(u16::MAX, u16::MAX))
            .0 as f32
            - min_x,
        spell
            .iter()
            .max_by_key(|(_x, y)| y)
            .unwrap_or(&(u16::MAX, u16::MAX))
            .1 as f32
            - min_y,
    );
    info!("w: {w}, h: {h}");
    // let spell = resample(
    //     spell
    //         .into_iter()
    //         .map(|(x, y)| (*x as f32, *y as f32))
    //         .collect(),
    // )
    // .await;

    (
        spell
            .into_iter()
            .map(move |(x, y)| ((*x as f32 - min_x) / w, (*y as f32 - min_y) / h))
            .collect(),
        w / h,
        w * h,
    )
}

async fn do_spell_compare(
    drawn: NormedSpell,
    compare: NormedSpell,
    // drawn_ar: f32,
    // compare_ar: f32,
) -> f32 {
    // info!("len_1 = {}, len_2 = {}", drawn.len(), compare.len());
    // let len = (drawn.len() + compare.len()) / 2;
    let len = compare.len().max(drawn.len());
    // info!("len = {len}");

    // let (drawn, compare) = (
    //     interpolate(drawn, len).await,
    //     interpolate(compare, len).await,
    // );

    let (drawn, compare) = (
        angle_vec(resample(interpolate(drawn, len).await).await).await,
        angle_vec(resample(interpolate(compare, len).await).await).await,
    );
    // info!("resample success");

    sum_squared_errors(drawn, compare).await

    // sum_squared_errors(drawn, compare).await
}

async fn interpolate(spell: NormedSpell, len: usize) -> NormedSpell {
    if len == spell.len() {
        return spell;
    }

    let ratio = spell.len() as f32 / len as f32;
    info!("ratio: {ratio}");
    info!("len: {len}");
    info!("ratio * len: {}", ratio * (len - 1) as f32);

    let futures = (0..len).map({
        move |i| {
            let percise_i = i as f32 * ratio;
            let i_1 = percise_i.floor() as usize;
            let i_2 = percise_i.ceil() as usize;
            let i_1 = if i_1 >= spell.len() {
                spell.len() - 2
            } else {
                i_1
            };
            let i_2 = if i_2 >= spell.len() {
                spell.len() - 1
            } else {
                i_2
            };
            let fract = percise_i.fract();
            let p_1 = spell[i_1];
            let p_2 = spell[i_2];
            lerp_2d(p_1, p_2, fract)
        }
    });

    collect_async(futures.collect()).await
}

pub async fn collect_async<T>(futures: Vec<impl Future<Output = T>>) -> Vec<T> {
    let mut val = Vec::with_capacity(futures.len());

    for fut in futures.into_iter() {
        let res = fut.await;

        val.push(res);
    }

    val
}

async fn sum_squared_errors(spell_1: NormedSpell, spell_2: NormedSpell) -> f32 {
    let keep_sign = |num: f32| {
        if num < 0.0 {
            num.abs().powf(2.0) * -1.0
        } else {
            num.abs().powf(2.0)
        }
    };

    let squared_errors: Vec<f32> = spell_1
        .into_iter()
        .zip(spell_2.into_iter())
        .map(|((x_1, y_1), (x_2, y_2))| {
            let a = x_2 - x_1;
            let b = y_2 - y_1;
            keep_sign(keep_sign(a) + keep_sign(b))
            // .sqrt()
            // .powf(2.0)
        })
        .collect();

    let sum: f32 = squared_errors.iter().sum();

    sum.abs() / (squared_errors.len() as f32)
}

// async fn sum_squared_errors_2(spell_1: NormedSpell, spell_2: NormedSpell) -> f32 {
//     let squared_errors: Vec<f32> = spell_1
//         .into_iter()
//         .zip(spell_2.into_iter())
//         .map(|((a1, _m1), (a2, _m2))| (a2 - a1).powf(2.0))
//         .collect();
//
//     let sum: f32 = squared_errors.iter().sum();
//
//     sum / (squared_errors.len() as f32)
// }

async fn lerp_2d(p_1: (f32, f32), p_2: (f32, f32), fract: f32) -> (f32, f32) {
    let lerp = |start, end, t| start + t * (end - start);

    let x = lerp(p_1.0, p_1.1, fract);
    let x = if x <= 1.0 { x } else { 1.0 };

    let y = lerp(p_2.0, p_2.1, fract);
    let y = if y <= 1.0 { y } else { 1.0 };

    (x, y)
}

async fn resample(spell: NormedSpell) -> NormedSpell {
    let distances: Vec<f32> = spell
        .iter()
        .zip(spell[1..].iter())
        .map(|((x_1, y_1), (x_2, y_2))| {
            ((x_2 - x_1).powf(2.0) + (y_2 - y_1).powf(2.0)).abs().sqrt()
        })
        .collect();
    let total_distance: f32 = distances.iter().sum();
    let len = distances.len();
    let avg_distance = total_distance / len as f32;
    info!("avg_distance: {total_distance} / {len} = {avg_distance}");
    // info!("{distances:?}");
    info!("{:?}, {:?}", spell[0], spell[1]);
    // info!(
    //     "{:?}",
    //     spell.iter().zip(spell[1..].iter()).collect::<Vec<_>>()[0..100].iter()
    // );
    // let ratio = avg_distance * distances[1];
    // info!("sample ratio {ratio}");
    if avg_distance < 1.0 {
        return spell;
    }

    let ratio = avg_distance / len as f32;
    info!("resample ratio: {ratio}");
    info!("resample len: {len}");
    info!("resample ratio * len: {}", ratio * (len - 1) as f32);

    let futures = // spell
        // .iter()
        // .zip(spell[1..].iter())
        // distances
        // .into_iter()
        // .enumerate()
        (0..len)
        .map({
            // let ratio = avg_distance;
            // info!();

            // |(i, distance)| {
            |i| {
                // let ratio = if avg_distance < distance {
                //     avg_distance / distance
                // } else if avg_distance > distance {
                //     distance / avg_distance
                // } else {
                //     1.0
                // };
                // let ratio = avg_distance * len as f32;
                let percise_i = i as f32 * ratio;
                let i_1 = percise_i.floor() as usize;
                let i_2 = percise_i.ceil() as usize;
                // let i_1 = if i_1 >= spell.len() {
                //     spell.len() - 1
                // } else {
                //     i_1
                // };
                let i_2 = if i_2 >= len {
                    len - 1
                } else {
                    i_2
                };
                let fract = percise_i.fract();
                let p_1 = spell[i_1];
                let p_2 = spell[i_2];
                lerp_2d(p_1, p_2, fract)
            }
        });

    collect_async(futures.collect()).await
}

async fn angle_vec(spell: NormedSpell) -> NormedSpell {
    spell
        .iter()
        .zip(spell[1..].iter())
        .map(|((x_1, y_1), (x_2, y_2))| {
            (
                ((y_2 - y_1).atan2(x_2 - x_1) / (PI * 2.)),
                ((x_2 - x_1).powf(2.0) + (y_2 - y_1).powf(2.0)).abs().sqrt(),
            )
        })
        .collect()
}
