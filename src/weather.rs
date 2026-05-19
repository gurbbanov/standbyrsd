use crate::l10n::Locale;
use crate::settings::{SpeedUnit, TemperatureUnit};
use iana_time_zone::get_timezone;
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Weather {
    pub city: Option<String>,
    pub coordinate: Option<(String, String)>,
    pub current: Option<CurrentForecast>,
    pub daily: Option<DailyForecast>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GeoResult {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

#[derive(Deserialize)]
pub struct GeoResponse {
    pub results: Option<Vec<GeoResult>>,
}

impl Weather {
    pub async fn fetch(
        &mut self,
        lang: &Locale,
        temp_unit: &TemperatureUnit,
        speed_unit: &SpeedUnit,
    ) -> Result<(), reqwest::Error> {
        let tz = get_timezone().unwrap_or("UTC".to_string());
        let city_hint = tz.split('/').last().unwrap_or("UTC").replace('_', " ");

        let geo = reqwest::get(format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&language={}&count=1",
            city_hint,
            lang.as_str()
        ))
        .await?
        .json::<GeoResponse>()
        .await?
        .results
        .and_then(|r| r.into_iter().next());

        let (lat, lon, name) = match geo {
            Some(r) => (r.latitude, r.longitude, r.name),
            None => (0.0, 0.0, String::from("Unknown")),
        };

        let temp_param = match temp_unit {
            TemperatureUnit::Celsius => "celsius",
            TemperatureUnit::Fahrenheit => "fahrenheit",
        };

        let speed_param = match speed_unit {
            SpeedUnit::KmH => "kmh",
            SpeedUnit::Ms => "ms",
            SpeedUnit::Mph => "mph",
        };

        let response: Weather = reqwest::get(
            format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=precipitation_probability_max,apparent_temperature_max,apparent_temperature_min,weather_code,uv_index_max,sunset,sunrise,daylight_duration&current=temperature_2m,is_day,wind_speed_10m,precipitation,weather_code,apparent_temperature&past_days=0&forecast_days=7&timezone=auto&language={}&temperature_unit={}&wind_speed_unit={}", lat, lon, lang, temp_param, speed_param),
        )
        .await?
        .json::<Self>()
        .await?;

        *self = Weather {
            city: Some(name),
            coordinate: Some((lat.to_string(), lon.to_string())),
            ..response
        };

        Ok(())
    }

    pub async fn fetch_for_city(
        result: &GeoResult,
        lang: &Locale,
    ) -> Result<Weather, reqwest::Error> {
        let response: Weather = reqwest::get(format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=precipitation_probability_max,apparent_temperature_max,apparent_temperature_min,weather_code,uv_index_max,sunset,sunrise,daylight_duration&current=temperature_2m,is_day,wind_speed_10m,precipitation,weather_code,apparent_temperature&past_days=0&forecast_days=7&timezone=auto&language={}",
            result.latitude, result.longitude, lang
        ))
        .await?
        .json::<Self>()
        .await?;

        Ok(Weather {
            city: Some(result.name.clone()),
            coordinate: Some((result.latitude.to_string(), result.longitude.to_string())),
            ..response
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CurrentForecast {
    pub interval: i32,
    pub is_day: u8,
    pub precipitation: f32,
    pub temperature_2m: f32,
    pub wind_speed_10m: f32,
    pub weather_code: u8,
    pub apparent_temperature: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DailyForecast {
    pub apparent_temperature_max: Vec<f32>,
    pub apparent_temperature_min: Vec<f32>,
    pub precipitation_probability_max: Vec<f32>,
    pub weather_code: Vec<u8>,
    pub uv_index_max: Vec<f32>,
    pub sunrise: Vec<String>,
    pub sunset: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub enum WeatherStatus {
    #[default]
    Loading,
    Ok(Weather),
    Error(String),
}
