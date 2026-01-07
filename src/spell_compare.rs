use alloc::vec::Vec;
use log::*;
use num_traits::Float;

use crate::Spell;

type NormedSpell = Vec<(f32, f32)>;

pub async fn spell_compare(drawn: &Spell, against: &Spell) -> f32 {
    let ((drawn, drawn_ar), (compare, compare_ar)) =
        (normalize(drawn).await, normalize(against).await);
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

    line_differnece * 0.8 + ar_diff * 0.2
}

pub async fn normalize(spell: &Spell) -> (NormedSpell, f32) {
    let min_x = spell.iter().min_by_key(|(x, _y)| x).unwrap_or(&(0, 0)).0;
    let min_y = spell.iter().min_by_key(|(_x, y)| y).unwrap_or(&(0, 0)).1;
    info!("min_x: {min_x}, min_y: {min_y}");
    let (w, h) = (
        (spell
            .iter()
            .max_by_key(|(x, _y)| x)
            .unwrap_or(&(u16::MAX, u16::MAX))
            .0
            - min_x) as f32,
        (spell
            .iter()
            .max_by_key(|(_x, y)| y)
            .unwrap_or(&(u16::MAX, u16::MAX))
            .1
            - min_y) as f32,
    );
    info!("w: {w}, h: {h}");

    (
        spell
            .into_iter()
            .map(move |(x, y)| ((x - min_x) as f32 / w, (y - min_y) as f32 / w))
            .collect(),
        w / h,
    )
}

async fn do_spell_compare(
    drawn: NormedSpell,
    compare: NormedSpell,
    // drawn_ar: f32,
    // compare_ar: f32,
) -> f32 {
    // info!("len_1 = {}, len_2 = {}", drawn.len(), compare.len());
    let len = (drawn.len() + compare.len()) / 2;
    // info!("len = {len}");

    let (drawn, compare) = (resample(drawn, len).await, resample(compare, len).await);
    // info!("resample success");

    sum_squared_errors(drawn, compare).await
}

async fn resample(spell: NormedSpell, len: usize) -> NormedSpell {
    // let lerp = |start, end, t| start + t * (end - start);

    let futures = (0..len).map({
        let ratio = if spell.len() <= len {
            spell.len() as f32 / len as f32
        } else {
            len as f32 / spell.len() as f32
        };
        info!("ratio: {ratio}");

        move |i| {
            let percise_i = i as f32 * ratio;
            let i_1 = percise_i.floor() as usize;
            let i_2 = percise_i.ceil() as usize;
            // let i_1 = if i_1 >= spell.len() {
            //     spell.len() - 1
            // } else {
            //     i_1
            // };
            let i_2 = if i_2 >= spell.len() {
                spell.len() - 1
            } else {
                i_2
            };
            let fract = percise_i.fract();
            let p_1 = spell[i_1];
            let p_2 = spell[i_2];
            do_resample(p_1, p_2, fract)
        }
    });

    let mut val = Vec::with_capacity(len);

    for fut in futures.into_iter() {
        let res = fut.await;

        val.push(res);
    }

    val
}

async fn sum_squared_errors(spell_1: NormedSpell, spell_2: NormedSpell) -> f32 {
    let squared_errors: Vec<f32> = spell_1
        .into_iter()
        .zip(spell_2.into_iter())
        .map(|((x_1, y_1), (x_2, y_2))| (x_2 - x_1).powf(2.0) + (y_2 - y_1).powf(2.0))
        .collect();

    let sum: f32 = squared_errors.iter().sum();

    sum / (squared_errors.len() as f32)
}

async fn do_resample(p_1: (f32, f32), p_2: (f32, f32), fract: f32) -> (f32, f32) {
    let lerp = |start, end, t| start + t * (end - start);

    let x = lerp(p_1.0, p_1.1, fract);
    let x = if x <= 1.0 { x } else { 1.0 };

    let y = lerp(p_2.0, p_2.1, fract);
    let y = if y <= 1.0 { y } else { 1.0 };

    (x, y)
}
