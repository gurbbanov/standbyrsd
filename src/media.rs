use chrono::{DateTime, Utc};
use iced::Color;

#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub position: i64,
    pub duration: i64,
    pub is_playing: bool,
    pub thumbnail: Option<iced::widget::image::Handle>,
    pub gradient_colors: Option<(Color, Color)>,
    pub position_origin: DateTime<Utc>,
}

pub fn extract_dominant_colors(buf: &[u8], theme_name: &str) -> (Color, Color) {
    let img = image::load_from_memory(buf).unwrap().to_rgb8();

    let pixels: Vec<[f32; 3]> = img
        .pixels()
        .step_by(10)
        .map(|p| {
            [
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
            ]
        })
        .collect();

    let mut c1 = pixels[0];
    let mut c2 = pixels[pixels.len() - 1];

    for _ in 0..20 {
        let mut sum1 = [0.0f32; 3];
        let mut sum2 = [0.0f32; 3];
        let mut count1 = 0usize;
        let mut count2 = 0usize;

        for p in &pixels {
            let d1 = dist(p, &c1);
            let d2 = dist(p, &c2);
            if d1 < d2 {
                sum1[0] += p[0];
                sum1[1] += p[1];
                sum1[2] += p[2];
                count1 += 1;
            } else {
                sum2[0] += p[0];
                sum2[1] += p[1];
                sum2[2] += p[2];
                count2 += 1;
            }
        }

        if count1 > 0 {
            c1 = [
                sum1[0] / count1 as f32,
                sum1[1] / count1 as f32,
                sum1[2] / count1 as f32,
            ];
        }
        if count2 > 0 {
            c2 = [
                sum2[0] / count2 as f32,
                sum2[1] / count2 as f32,
                sum2[2] / count2 as f32,
            ];
        }
    }

    let darken = |c: [f32; 3]| {
        if theme_name == "classic" {
            let min = 0.15f32;
            Color::from_rgb(
                (c[0] * 0.6).max(min),
                (c[1] * 0.6).max(min),
                (c[2] * 0.6).max(min),
            )
        } else {
            let luma = c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
            let l = (luma * 0.3).min(0.25);
            Color::from_rgb((l * 2.5).max(0.15).min(0.5), l * 0.3, l * 0.3)
        }
    };

    (darken(c1), darken(c2))
}

fn dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}
